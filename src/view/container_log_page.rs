use std::borrow::Cow;
use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::BufWriter;
use std::io::Write;
use std::mem;
use std::sync::OnceLock;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use adw::prelude::*;
use adw::subclass::prelude::*;
use ashpd::WindowIdentifier;
use ashpd::desktop::file_chooser::Choice;
use ashpd::desktop::file_chooser::SaveFileRequest;
use futures::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use sourceview5::prelude::*;

use crate::model;
use crate::podman;
use crate::utils;
use crate::widget;

const ACTION_TOGGLE_SEARCH: &str = "container-log-page.toggle-search";
const ACTION_EXIT_SEARCH: &str = "container-log-page.exit-search";
const ACTION_SAVE_TO_FILE: &str = "container-log-page.save-to-file";
const ACTION_SHOW_TIMESTAMPS: &str = "container-log-page.show-timestamps";
const ACTION_SCROLL_DOWN: &str = "container-log-page.scroll-down";
const ACTION_START_CONTAINER: &str = "container-log-page.start-container";
const ACTION_ZOOM_OUT: &str = "container-log-page.zoom-out";
const ACTION_ZOOM_IN: &str = "container-log-page.zoom-in";
const ACTION_ZOOM_NORMAL: &str = "container-log-page.zoom-normal";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum FetchLinesState {
    #[default]
    Waiting,
    Fetching,
    Finished,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerLogPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_log_page.ui")]
    pub(crate) struct ContainerLogPage {
        pub(super) settings: utils::PodsSettings,
        pub(super) log_timestamps: RefCell<VecDeque<String>>,
        pub(super) fetch_until: OnceCell<String>,
        pub(super) fetch_lines_state: Cell<FetchLinesState>,
        pub(super) fetched_lines: RefCell<VecDeque<Vec<u8>>>,
        pub(super) prev_adj: Cell<f64>,
        pub(super) is_auto_scrolling: Cell<bool>,
        #[property(get, set, construct, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[property(get, set)]
        pub(super) sticky: Cell<bool>,
        #[template_child]
        pub(super) zoom_control: TemplateChild<widget::ZoomControl>,
        #[template_child]
        pub(super) timestamps_renderer: TemplateChild<sourceview5::GutterRendererText>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub(super) search_widget: TemplateChild<widget::SourceViewSearchWidget>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) lines_loading_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub(super) scalable_text_view: TemplateChild<widget::ScalableTextView>,
        #[template_child]
        pub(super) source_buffer: TemplateChild<sourceview5::Buffer>,
        #[template_child]
        pub(super) banner: TemplateChild<adw::Banner>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerLogPage {
        const NAME: &'static str = "PdsContainerLogPage";
        type Type = super::ContainerLogPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.add_binding_action(
                gdk::Key::F,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_TOGGLE_SEARCH,
            );
            klass.install_action(ACTION_TOGGLE_SEARCH, None, |widget, _, _| {
                widget.toggle_search_mode();
            });

            klass.add_binding_action(
                gdk::Key::Escape,
                gdk::ModifierType::empty(),
                ACTION_EXIT_SEARCH,
            );
            klass.install_action(ACTION_EXIT_SEARCH, None, |widget, _, _| {
                widget.set_search_mode(false);
            });

            klass.install_action_async(ACTION_SAVE_TO_FILE, None, |widget, _, _| async move {
                widget.save_to_file().await;
            });
            klass.install_property_action(ACTION_SHOW_TIMESTAMPS, "show-timestamps");

            klass.install_action(ACTION_SCROLL_DOWN, None, |widget, _, _| {
                widget.scroll_down();
            });
            klass.install_action(ACTION_START_CONTAINER, None, |widget, _, _| {
                widget.start_or_resume_container();
            });

            klass.install_action(ACTION_ZOOM_OUT, None, |widget, _, _| {
                widget.imp().scalable_text_view.zoom_out();
            });
            klass.install_action(ACTION_ZOOM_IN, None, |widget, _, _| {
                widget.imp().scalable_text_view.zoom_in();
            });
            klass.install_action(ACTION_ZOOM_NORMAL, None, |widget, _, _| {
                widget.imp().scalable_text_view.zoom_normal();
            });

            klass.add_binding_action(
                gdk::Key::minus,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_ZOOM_OUT,
            );
            klass.add_binding_action(
                gdk::Key::KP_Subtract,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_ZOOM_OUT,
            );

            klass.add_binding_action(
                gdk::Key::plus,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_ZOOM_IN,
            );
            klass.add_binding_action(
                gdk::Key::KP_Add,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_ZOOM_IN,
            );
            klass.add_binding_action(
                gdk::Key::equal,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_ZOOM_IN,
            );

            klass.add_binding_action(
                gdk::Key::_0,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_ZOOM_NORMAL,
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerLogPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                Self::derived_properties()
                    .iter()
                    .cloned()
                    .chain(Some(
                        glib::ParamSpecBoolean::builder("show-timestamps")
                            .explicit_notify()
                            .build(),
                    ))
                    .collect::<Vec<_>>()
            })
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "show-timestamps" => self.obj().set_show_timestamps(value.get().unwrap()),
                _ => self.derived_set_property(id, value, pspec),
            }
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "show-timestamps" => self.obj().show_timestamps().to_value(),
                _ => self.derived_property(id, pspec),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            self.settings
                .bind(
                    "show-log-timestamps",
                    &self.timestamps_renderer.get(),
                    "visible",
                )
                .build();

            self.menu_button
                .popover()
                .unwrap()
                .downcast::<gtk::PopoverMenu>()
                .unwrap()
                .add_child(&*self.zoom_control, "zoom-control");

            let adw_style_manager = adw::StyleManager::default();
            obj.on_notify_dark(&adw_style_manager);
            adw_style_manager.connect_dark_notify(clone!(
                #[weak]
                obj,
                move |style_manager| {
                    obj.on_notify_dark(style_manager);
                }
            ));

            <widget::ScalableTextView as ViewExt>::gutter(
                &*self.scalable_text_view,
                gtk::TextWindowType::Left,
            )
            .insert(&self.timestamps_renderer.get(), 0);

            let mut maybe_gutter_child = <widget::ScalableTextView as ViewExt>::gutter(
                &*self.scalable_text_view,
                gtk::TextWindowType::Left,
            )
            .first_child();

            while let Some(child) = maybe_gutter_child {
                if child.is::<sourceview5::GutterRenderer>() {
                    child.set_margin_start(4);
                }

                maybe_gutter_child = child.next_sibling()
            }

            let adj = self.scrolled_window.vadjustment();
            obj.on_adjustment_changed(&adj);
            adj.connect_value_changed(clone!(
                #[weak]
                obj,
                move |adj| {
                    obj.on_adjustment_changed(adj);
                }
            ));

            adj.connect_upper_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    if obj.sticky() || obj.imp().is_auto_scrolling.get() {
                        obj.scroll_down();
                    }
                }
            ));

            Self::Type::this_expression("container")
                .chain_property::<model::Container>("status")
                .chain_closure::<bool>(closure!(|_: Self::Type, status: model::ContainerStatus| {
                    status != model::ContainerStatus::Running
                }))
                .bind(&*self.banner, "revealed", Some(obj));

            if let Some(container) = obj.container() {
                container.connect_notify_local(
                    Some("status"),
                    clone!(
                        #[weak]
                        obj,
                        move |container, _| {
                            if container.status() == model::ContainerStatus::Running {
                                obj.follow_log();
                            }
                        }
                    ),
                );
            }

            obj.init_log();
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ContainerLogPage {}

    #[gtk::template_callbacks]
    impl ContainerLogPage {
        #[template_callback]
        fn on_timestamps_renderer_query_data(&self, _: &glib::Object, line: u32) {
            if let Some(timestamp) = self.log_timestamps.borrow().get(line as usize) {
                let date_time = format!("<span foreground=\"#865e3c\">{timestamp}</span>",);
                self.timestamps_renderer.set_markup(&date_time);

                let (width, _) = self.timestamps_renderer.measure_markup(&date_time);
                self.timestamps_renderer
                    .set_width_request(width.max(self.timestamps_renderer.width_request()));
            }
        }

        #[template_callback]
        fn on_timestamps_renderer_notify_visible(&self) {
            self.obj().notify("show-timestamps");
        }

        #[template_callback]
        fn on_scroll(
            &self,
            _dx: f64,
            dy: f64,
            scroll: gtk::EventControllerScroll,
        ) -> glib::Propagation {
            if scroll.current_event_state() == gdk::ModifierType::CONTROL_MASK {
                let text_view = &*self.scalable_text_view;
                if dy.is_sign_negative() {
                    text_view.zoom_in();
                } else {
                    text_view.zoom_out();
                }
            }

            glib::Propagation::Proceed
        }

        #[template_callback]
        fn on_source_buffer_cursor_moved(&self) {
            self.timestamps_renderer.queue_draw();
        }

        #[template_callback]
        fn on_search_bar_search_mode_enabled(&self) {
            if self.search_bar.is_search_mode() {
                self.search_widget.grab_focus();
            } else {
                self.search_widget.set_text("");
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerLogPage(ObjectSubclass<imp::ContainerLogPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for ContainerLogPage {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder()
            .property("container", container)
            .build()
    }
}

impl ContainerLogPage {
    pub(crate) fn show_timestamps(&self) -> bool {
        self.imp().timestamps_renderer.is_visible()
    }

    pub(crate) fn set_show_timestamps(&self, value: bool) {
        if self.show_timestamps() == value {
            return;
        }

        self.imp().timestamps_renderer.set_visible(value);
    }

    pub(crate) fn scroll_down(&self) {
        let imp = self.imp();

        imp.is_auto_scrolling.set(true);
        glib::idle_add_local_once(clone!(
            #[weak(rename_to = obj)]
            self,
            move || {
                obj.imp().scrolled_window.vadjustment().set_value(f64::MAX);
            }
        ));
    }

    fn on_adjustment_changed(&self, adj: &gtk::Adjustment) {
        let imp = self.imp();

        if imp.is_auto_scrolling.get() {
            if adj.value() + adj.page_size() >= adj.upper() {
                imp.is_auto_scrolling.set(false);
                self.set_sticky(true);
            }
        } else {
            self.set_sticky(adj.value() + adj.page_size() >= adj.upper());
            self.load_previous_messages(adj);
        }

        imp.prev_adj.replace(adj.value());
    }

    fn init_log(&self) {
        if let Some(container) = self.container().as_ref().and_then(model::Container::api) {
            let mut perform = MarkupPerform::default();

            utils::run_stream_with_finish_handler(
                container,
                move |container| {
                    container
                        .logs(&basic_opts_builder(false, true).tail("512").build())
                        .boxed()
                },
                clone!(
                    #[weak(rename_to = obj)]
                    self,
                    #[upgrade_or]
                    glib::ControlFlow::Break,
                    move |result| {
                        obj.imp().stack.set_visible_child_name("loaded");
                        obj.append_line(result, &mut perform)
                    }
                ),
                clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move || {
                        obj.imp().stack.set_visible_child_name("loaded");
                        obj.follow_log();
                    }
                ),
            );
        }
    }

    fn follow_log(&self) {
        if let Some(container) = self.container().as_ref().and_then(model::Container::api) {
            let timestamps = self.imp().log_timestamps.borrow();
            let mut iter = timestamps.iter().rev();

            let opts = basic_opts_builder(true, true);
            let (opts, skip) = match iter.next() {
                Some(last) => (
                    opts.since(
                        glib::DateTime::from_iso8601(last, None)
                            .unwrap()
                            .to_unix()
                            .to_string(),
                    ),
                    AtomicUsize::new(iter.take_while(|t| *t == last).count() + 1),
                ),
                None => (opts, AtomicUsize::new(0)),
            };

            let mut perform = MarkupPerform::default();

            utils::run_stream(
                container,
                move |container| container.logs(&opts.build()).boxed(),
                clone!(
                    #[weak(rename_to = obj)]
                    self,
                    #[upgrade_or]
                    glib::ControlFlow::Break,
                    move |result: podman::Result<podman::conn::TtyChunk>| {
                        if skip.load(Ordering::Relaxed) == 0 {
                            obj.append_line(result, &mut perform)
                        } else {
                            skip.fetch_sub(1, Ordering::Relaxed);
                            glib::ControlFlow::Continue
                        }
                    }
                ),
            );
        }
    }

    fn append_line(
        &self,
        result: podman::Result<podman::conn::TtyChunk>,
        perform: &mut MarkupPerform,
    ) -> glib::ControlFlow {
        match result {
            Ok(line) => {
                self.insert(Vec::from(line), perform, true);
                glib::ControlFlow::Continue
            }
            Err(e) => {
                log::warn!("Stopping container log stream due to error: {e}");
                utils::show_error_toast(
                    self,
                    &gettext("Error while following log"),
                    &e.to_string(),
                );
                glib::ControlFlow::Break
            }
        }
    }

    fn insert(&self, line: Vec<u8>, perform: &mut MarkupPerform, at_end: bool) {
        let imp = self.imp();

        let line_buffer = perform.decode(&line);
        let (timestamp, log_message) = line_buffer.split_once(' ').unwrap();

        imp.fetch_until.get_or_init(|| timestamp.to_owned());

        let source_buffer = &*imp.source_buffer;
        source_buffer.insert_markup(
            &mut if at_end {
                imp.source_buffer.end_iter()
            } else {
                imp.source_buffer.start_iter()
            },
            &if source_buffer.start_iter() == source_buffer.end_iter() {
                Cow::Borrowed(log_message)
            } else if at_end {
                Cow::Owned(format!("\n{log_message}"))
            } else {
                Cow::Owned(format!("{log_message}\n"))
            },
        );

        let mut timestamps = imp.log_timestamps.borrow_mut();
        if at_end {
            timestamps.push_back(timestamp.to_owned());
        } else {
            timestamps.push_front(timestamp.to_owned());
        }
    }

    fn load_previous_messages(&self, adj: &gtk::Adjustment) {
        let imp = self.imp();

        if adj.value() >= imp.prev_adj.get() || adj.value() >= adj.page_size() {
            return;
        }

        match imp.fetch_lines_state.get() {
            FetchLinesState::Waiting => {
                if let Some(until) = imp.fetch_until.get().map(ToOwned::to_owned) {
                    if let Some(container) =
                        self.container().as_ref().and_then(model::Container::api)
                    {
                        imp.lines_loading_revealer.set_reveal_child(true);

                        utils::run_stream_with_finish_handler(
                            container,
                            move |container| {
                                container
                                    .logs(&basic_opts_builder(false, true).until(until).build())
                                    .boxed()
                            },
                            clone!(
                                #[weak(rename_to = obj)]
                                self,
                                #[upgrade_or]
                                glib::ControlFlow::Break,
                                move |result| {
                                    let imp = obj.imp();
                                    imp.fetch_lines_state.set(FetchLinesState::Fetching);

                                    match result {
                                        Ok(line) => {
                                            imp.fetched_lines
                                                .borrow_mut()
                                                .push_back(Vec::from(line));
                                            glib::ControlFlow::Continue
                                        }
                                        Err(e) => {
                                            log::warn!(
                                                "Stopping container log stream due to error: {e}"
                                            );
                                            glib::ControlFlow::Break
                                        }
                                    }
                                }
                            ),
                            clone!(
                                #[weak(rename_to = obj)]
                                self,
                                move || {
                                    let imp = obj.imp();
                                    imp.lines_loading_revealer.set_reveal_child(false);
                                    imp.fetch_lines_state.set(FetchLinesState::Finished);

                                    obj.move_lines_to_buffer();
                                }
                            ),
                        );
                    }
                }
            }
            FetchLinesState::Finished => self.move_lines_to_buffer(),
            _ => {}
        }
    }

    fn move_lines_to_buffer(&self) {
        let mut perform = MarkupPerform::default();

        let imp = self.imp();
        let mut lines = imp.fetched_lines.borrow_mut();

        let had_lines = !lines.is_empty();

        for _ in 0..128 {
            match lines.pop_back() {
                Some(line) => self.insert(line, &mut perform, false),
                None => break,
            }
        }

        if had_lines {
            let adj = imp.scrolled_window.vadjustment();
            if adj.value() < 30.0 {
                adj.set_value(adj.value() + 30.0);
            }
        }
    }

    fn on_notify_dark(&self, style_manager: &adw::StyleManager) {
        self.imp().source_buffer.set_style_scheme(
            sourceview5::StyleSchemeManager::default()
                .scheme(if style_manager.is_dark() {
                    "Adwaita-dark"
                } else {
                    "Adwaita"
                })
                .as_ref(),
        );
    }

    async fn save_to_file(&self) {
        if let Some(container) = self.container() {
            let request = SaveFileRequest::default()
                .identifier(WindowIdentifier::from_native(&self.native().unwrap()).await)
                .current_name(format!("{}.log", container.name()).as_str())
                .choice(Choice::boolean(
                    "timestamps",
                    &gettext("Include timestamps"),
                    false,
                ))
                .modal(true);

            utils::show_save_file_dialog(
                request,
                self,
                clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |files| {
                        obj.action_set_enabled(ACTION_SAVE_TO_FILE, false);

                        let file = gio::File::for_uri(files.uris()[0].as_str());

                        if let Some(path) = file.path() {
                            let file = std::fs::OpenOptions::new()
                                .write(true)
                                .create(true)
                                .truncate(true)
                                .open(path)
                                .unwrap();

                            let mut writer = BufWriter::new(file);
                            let mut perform = PlainTextPerform::default();

                            let timestamps = files.choices()[0].1 == "true";

                            utils::run_stream_with_finish_handler(
                                container.api().unwrap(),
                                move |container| {
                                    container
                                        .logs(&basic_opts_builder(false, timestamps).build())
                                        .boxed()
                                },
                                clone!(
                                    #[weak]
                                    obj,
                                    #[upgrade_or]
                                    glib::ControlFlow::Break,
                                    move |result: podman::Result<podman::conn::TtyChunk>| {
                                        match result.map(Vec::from) {
                                            Ok(line) => {
                                                perform.decode(&line);

                                                let line = perform.move_out_buffer();
                                                if !line.is_empty() {
                                                    match writer
                                                        .write_all(line.as_bytes())
                                                        .and_then(|_| writer.write_all(b"\n"))
                                                    {
                                                        Ok(_) => glib::ControlFlow::Continue,
                                                        Err(e) => {
                                                            log::warn!("Error on saving logs: {e}");
                                                            utils::show_error_toast(
                                                                &obj,
                                                                &gettext("Error on saving logs"),
                                                                &e.to_string(),
                                                            );
                                                            glib::ControlFlow::Break
                                                        }
                                                    }
                                                } else {
                                                    glib::ControlFlow::Continue
                                                }
                                            }
                                            Err(e) => {
                                                log::warn!("Error on retrieving logs: {e}");
                                                utils::show_error_toast(
                                                    &obj,
                                                    &gettext("Error on retrieving logs"),
                                                    &e.to_string(),
                                                );
                                                glib::ControlFlow::Break
                                            }
                                        }
                                    }
                                ),
                                clone!(
                                    #[weak]
                                    obj,
                                    move || {
                                        obj.action_set_enabled(ACTION_SAVE_TO_FILE, true);
                                        utils::show_toast(&obj, gettext("Log has been saved"));
                                    }
                                ),
                            );
                        }
                    }
                ),
            )
            .await;
        }
    }

    pub(crate) fn set_search_mode(&self, value: bool) {
        self.imp().search_bar.set_search_mode(value);
    }

    pub(crate) fn toggle_search_mode(&self) {
        self.set_search_mode(!self.imp().search_bar.is_search_mode());
    }

    pub(crate) fn start_or_resume_container(&self) {
        if let Some(container) = self.container() {
            if container.can_start() {
                container.start(clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |result| {
                        if let Err(e) = result {
                            utils::show_error_toast(
                                &obj,
                                &gettext("Error starting container"),
                                &e.to_string(),
                            );
                        }
                    }
                ));
            } else if container.can_resume() {
                container.resume(clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |result| {
                        if let Err(e) = result {
                            utils::show_error_toast(
                                &obj,
                                &gettext("Error resuming container"),
                                &e.to_string(),
                            );
                        }
                    }
                ));
            }
        }
    }
}

fn basic_opts_builder(follow: bool, timestamps: bool) -> podman::opts::ContainerLogsOptsBuilder {
    podman::opts::ContainerLogsOpts::builder()
        .follow(follow)
        .stdout(true)
        .stderr(true)
        .timestamps(timestamps)
}

#[derive(Debug)]
enum MarkupAttribute {
    Bold,
    Foreground(&'static str),
    Background(&'static str),
}

impl MarkupAttribute {
    fn open_tag(&self) -> Cow<str> {
        match self {
            Self::Bold => Cow::Borrowed("<b>"),
            Self::Foreground(value) => Cow::Owned(format!("<span foreground=\"{value}\">")),
            Self::Background(value) => Cow::Owned(format!("<span background=\"{value}\">")),
        }
    }

    fn close_tag(&self) -> &'static str {
        match self {
            Self::Bold => "</b>",
            Self::Foreground(_) | Self::Background(_) => "</span>",
        }
    }
}

#[derive(Debug, Default)]
pub struct MarkupPerform {
    buffer: String,
    attributes: Vec<MarkupAttribute>,
}

impl MarkupPerform {
    fn move_out_buffer(&mut self) -> String {
        let mut buffer = String::new();
        mem::swap(&mut self.buffer, &mut buffer);
        buffer
    }

    fn begin_line(&mut self) {
        self.attributes.iter().for_each(|attr| {
            self.buffer.push_str(attr.open_tag().as_ref());
        });
    }

    fn end_line(&mut self) {
        self.attributes.iter().rev().for_each(|attr| {
            self.buffer.push_str(attr.close_tag());
        });
    }

    fn reset_all(&mut self) {
        while let Some(attr) = self.attributes.pop() {
            self.buffer.push_str(attr.close_tag());
        }
    }

    fn reset<F: Fn(&MarkupAttribute) -> bool>(&mut self, op: F) {
        let mut t = Vec::new();
        while let Some(attr) = self.attributes.pop() {
            self.buffer.push_str(attr.close_tag());
            if op(&attr) {
                t.insert(0, attr);
            }
        }

        mem::swap(&mut t, &mut self.attributes);

        self.begin_line();
    }

    /// Decode the specified bytes. Return true if finished.
    fn decode(&mut self, ansi_encoded_bytes: &[u8]) -> String {
        let mut parser = vte::Parser::new();

        self.begin_line();

        let line = String::from_utf8_lossy(ansi_encoded_bytes);
        let (timestamp, message) = line.split_once(' ').unwrap();

        parser.advance(self, message.as_bytes());

        self.end_line();

        format!("{timestamp} {}", self.move_out_buffer())
    }
}

impl vte::Perform for MarkupPerform {
    fn print(&mut self, c: char) {
        self.buffer.push(c);
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
        for param in params.iter() {
            param.iter().copied().for_each(|param| {
                match param {
                    0 => self.reset_all(),
                    39 => {
                        // Reset foreground
                        self.reset(|attr| !matches!(attr, MarkupAttribute::Foreground(_)));
                    }
                    49 => {
                        // Reset background
                        self.reset(|attr| !matches!(attr, MarkupAttribute::Background(_)));
                    }
                    _ => {
                        if let Some(attr) = ansi_escape_to_markup_attribute(param) {
                            self.buffer.push_str(attr.open_tag().as_ref());
                            self.attributes.push(attr);
                        }
                    }
                }
            });
        }
    }
}

fn ansi_escape_to_markup_attribute(item: u16) -> Option<MarkupAttribute> {
    Some(match item {
        1 => MarkupAttribute::Bold,

        30 => MarkupAttribute::Foreground("#000000"),
        31 => MarkupAttribute::Foreground("#e01b24"),
        32 => MarkupAttribute::Foreground("#33d17a"),
        33 => MarkupAttribute::Foreground("#f6d32d"),
        34 => MarkupAttribute::Foreground("#3584e4"),
        35 => MarkupAttribute::Foreground("#d4267e"),
        36 => MarkupAttribute::Foreground("#00f7f7"),
        37 => MarkupAttribute::Foreground("#ffffff"),

        40 => MarkupAttribute::Background("#000000"),
        41 => MarkupAttribute::Background("#e01b24"),
        42 => MarkupAttribute::Background("#33d17a"),
        43 => MarkupAttribute::Background("#f6d32d"),
        44 => MarkupAttribute::Background("#3584e4"),
        45 => MarkupAttribute::Background("#d4267e"),
        46 => MarkupAttribute::Background("#00f7f7"),
        47 => MarkupAttribute::Background("#ffffff"),

        90 => MarkupAttribute::Foreground("#3d3846"),
        91 => MarkupAttribute::Foreground("#f66151"),
        92 => MarkupAttribute::Foreground("#8ff0a4"),
        93 => MarkupAttribute::Foreground("#f9f06b"),
        94 => MarkupAttribute::Foreground("#99c1f1"),
        95 => MarkupAttribute::Foreground("#c061cb"),
        96 => MarkupAttribute::Foreground("#33c7de"),
        97 => MarkupAttribute::Foreground("#f66151"),

        100 => MarkupAttribute::Background("#3d3846"),
        101 => MarkupAttribute::Background("#f66151"),
        102 => MarkupAttribute::Background("#8ff0a4"),
        103 => MarkupAttribute::Background("#f9f06b"),
        104 => MarkupAttribute::Background("#99c1f1"),
        105 => MarkupAttribute::Background("#c061cb"),
        106 => MarkupAttribute::Background("#33c7de"),
        109 => MarkupAttribute::Background("#f66151"),

        _ => return None,
    })
}

#[derive(Debug, Default)]
pub struct PlainTextPerform(String);

impl PlainTextPerform {
    fn move_out_buffer(&mut self) -> String {
        let mut buffer = String::new();
        mem::swap(&mut self.0, &mut buffer);
        buffer
    }

    fn decode(&mut self, ansi_encoded_bytes: &[u8]) {
        let mut parser = vte::Parser::new();

        parser.advance(self, ansi_encoded_bytes);
        // String::from_utf8_lossy(ansi_encoded_bytes)
        //     .bytes()
        //     .for_each(|byte| parser.advance(self, byte));
    }
}

impl vte::Perform for PlainTextPerform {
    fn print(&mut self, c: char) {
        self.0.push(c);
    }
}
