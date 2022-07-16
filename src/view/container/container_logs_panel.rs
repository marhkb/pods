use std::borrow::Cow;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::mem;

use futures::stream;
use futures::StreamExt;
use gettextrs::gettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;
use sourceview5::traits::BufferExt;
use sourceview5::traits::GutterRendererExt;
use sourceview5::traits::GutterRendererTextExt;
use sourceview5::traits::SearchSettingsExt;
use sourceview5::traits::ViewExt;

use crate::api;
use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container-logs-panel.ui")]
    pub(crate) struct ContainerLogsPanel {
        pub(super) settings: utils::PodsSettings,
        pub(super) container: WeakRef<model::Container>,
        pub(super) renderer_timestamps: OnceCell<sourceview5::GutterRendererText>,
        pub(super) search_settings: sourceview5::SearchSettings,
        pub(super) search_context: OnceCell<sourceview5::SearchContext>,
        pub(super) search_iters: RefCell<Option<(gtk::TextIter, gtk::TextIter)>>,
        pub(super) abort_handle: RefCell<Option<stream::AbortHandle>>,
        pub(super) handler_id: RefCell<Option<glib::SignalHandlerId>>,
        pub(super) log_timestamps: RefCell<BTreeMap<u32, String>>,
        pub(super) is_auto_scrolling: Cell<bool>,
        pub(super) sticky: Cell<bool>,
        #[template_child]
        pub(super) search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub(super) search_entry: TemplateChild<view::TextSearchEntry>,
        #[template_child]
        pub(super) regex_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) case_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) word_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub(super) source_view: TemplateChild<sourceview5::View>,
        #[template_child]
        pub(super) source_buffer: TemplateChild<sourceview5::Buffer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerLogsPanel {
        const NAME: &'static str = "ContainerLogsPanel";
        type Type = super::ContainerLogsPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("logs.scroll-down", None, move |widget, _, _| {
                widget.scroll_down();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerLogsPanel {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "container",
                        "Container",
                        "The container of this ContainerLogsPanel",
                        model::Container::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecBoolean::new(
                        "sticky",
                        "Sticky",
                        "Whether the log should stick to the last message",
                        true,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "container" => obj.set_container(value.get().unwrap()),
                "sticky" => obj.set_sticky(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container" => obj.container().to_value(),
                "sticky" => obj.sticky().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let adw_style_manager = adw::StyleManager::default();
            obj.on_notify_dark(&adw_style_manager);
            adw_style_manager.connect_dark_notify(clone!(@weak obj => move |style_manager| {
                obj.on_notify_dark(style_manager);
            }));

            let renderer_timestamps = sourceview5::GutterRendererText::builder()
                .margin_end(6)
                .build();
            renderer_timestamps.connect_query_data(clone!(@weak obj => move |renderer, _, line| {
                let log_timestamps = obj.imp().log_timestamps.borrow_mut();
                let date_time = format!(
                    "<span foreground=\"#865e3c\">{}</span>",
                    log_timestamps.get(&line).unwrap()
                );
                renderer.set_markup(&date_time);

                let (width, _) = renderer.measure_markup(&date_time);
                renderer.set_width_request(width.max(renderer.width_request()));
            }));
            self.source_buffer.connect_cursor_moved(
                clone!(@weak renderer_timestamps => move |_| renderer_timestamps.queue_draw()),
            );
            <sourceview5::View as ViewExt>::gutter(&*self.source_view, gtk::TextWindowType::Left)
                .insert(&renderer_timestamps, 0);
            self.renderer_timestamps.set(renderer_timestamps).unwrap();

            let mut maybe_gutter_child = <sourceview5::View as ViewExt>::gutter(
                &*self.source_view,
                gtk::TextWindowType::Left,
            )
            .first_child();

            while let Some(child) = maybe_gutter_child {
                if child.is::<sourceview5::GutterRenderer>() {
                    child.set_margin_start(4);
                }

                maybe_gutter_child = child.next_sibling()
            }

            self.search_bar.connect_search_mode_enabled_notify(
                clone!(@weak obj => move |search_bar| {
                    let search_entry = &*obj.imp().search_entry;
                    if search_bar.is_search_mode() {
                        search_entry.grab_focus();
                    } else {
                        search_entry.set_text("");
                    }
                }),
            );

            self.search_entry
                .bind_property("text", &self.search_settings, "search-text")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.search_settings.set_wrap_around(true);

            self.regex_button
                .bind_property("active", &self.search_settings, "regex-enabled")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.case_button
                .bind_property("active", &self.search_settings, "case-sensitive")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.word_button
                .bind_property("active", &self.search_settings, "at-word-boundaries")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            let search_context =
                sourceview5::SearchContext::new(&*self.source_buffer, Some(&self.search_settings));

            search_context.connect_occurrences_count_notify(clone!(@weak obj => move |_| {
                obj.update_search_occurences()
            }));

            self.search_context.set(search_context).unwrap();

            let adj = self.scrolled_window.vadjustment();
            obj.on_adjustment_changed(&adj);
            adj.connect_value_changed(clone!(@weak obj => move |adj| {
                obj.on_adjustment_changed(adj);
            }));

            adj.connect_upper_notify(clone!(@weak obj => move |_| {
                if obj.sticky() || Self::from_instance(&obj).is_auto_scrolling.get() {
                    obj.scroll_down();
                }
            }));

            obj.scroll_down();
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.overlay.unparent();
        }
    }

    impl WidgetImpl for ContainerLogsPanel {}
}

glib::wrapper! {
    pub(crate) struct ContainerLogsPanel(ObjectSubclass<imp::ContainerLogsPanel>)
        @extends gtk::Widget;
}

impl ContainerLogsPanel {
    fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    fn set_container(&self, value: Option<&model::Container>) {
        let container = self.container();

        if container.as_ref() == value {
            return;
        }

        self.abort();

        let imp = self.imp();

        if let Some(container) = container {
            container.disconnect(imp.handler_id.take().unwrap());
        }

        if let Some(container) = value {
            self.connect_logs(container, None);

            let handler_id = container.connect_notify_local(
                Some("status"),
                clone!(@weak self as obj => move |container, _| {
                    if let model::ContainerStatus::Running = container.status() {
                        obj.abort();
                        obj.connect_logs(
                            container,
                            obj.imp()
                                .log_timestamps
                                .borrow()
                                .values()
                                .last()
                                .cloned()
                        );
                    }
                }),
            );
            imp.handler_id.replace(Some(handler_id));
        }

        imp.container.set(value);
        self.notify("container");
    }

    fn sticky(&self) -> bool {
        self.imp().sticky.get()
    }

    fn set_sticky(&self, sticky: bool) {
        if self.sticky() == sticky {
            return;
        }

        self.imp().sticky.set(sticky);
        self.notify("sticky");
    }

    fn scroll_down(&self) {
        let imp = self.imp();

        imp.is_auto_scrolling.set(true);
        imp.scrolled_window
            .emit_by_name::<bool>("scroll-child", &[&gtk::ScrollType::End, &false]);
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
        }
    }

    fn connect_logs(&self, container: &model::Container, since: Option<String>) {
        if let Some(container) = container.api_container() {
            let mut perform = MarkupPerform::default();

            utils::run_stream(
                container,
                move |container| {
                    let opts = api::ContainerLogsOpts::builder()
                        .follow(true)
                        .stdout(true)
                        .stderr(true)
                        .timestamps(true);
                    container
                        .logs(
                            &match since {
                                Some(date_time) => opts.since(date_time),
                                None => opts,
                            }
                            .build(),
                        )
                        .boxed()
                },
                clone!(@weak self as obj => @default-return glib::Continue(false), move |result: api::Result<Vec<u8>>| {
                    glib::Continue(match result {
                        Ok(line) => {
                            let imp = obj.imp();
                            if line.len() > 8 && perform.decode(&line[8..]) {
                                let line_buffer = perform.move_out_buffer();

                                if let Some((timestamp, log_message)) = line_buffer.split_once(' ') {
                                    let source_buffer = &*imp.source_buffer;
                                    source_buffer.insert_markup(
                                        &mut source_buffer.end_iter(),
                                        &if source_buffer.start_iter() == source_buffer.end_iter() {
                                            Cow::Borrowed(log_message)
                                        } else {
                                            Cow::Owned(format!("\n{}", log_message))
                                        },
                                    );

                                    imp.log_timestamps.borrow_mut().insert(
                                        source_buffer.line_count() as u32 - 1,
                                        timestamp.to_owned()
                                    );
                                }
                            }
                            true
                        }
                        Err(e) => {
                            log::warn!("Stopping container log stream due to error: {e}");
                            false
                        }
                    })
                }),
            );
        }
    }

    fn abort(&self) {
        if let Some(handle) = self.imp().abort_handle.take() {
            handle.abort();
        }
    }

    pub(crate) fn connect_show_timestamp_button(&self, show_timestamp_button: &gtk::ToggleButton) {
        let imp = self.imp();

        show_timestamp_button
            .bind_property(
                "active",
                &*imp.renderer_timestamps.get().unwrap(),
                "visible",
            )
            .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
            .build();

        imp.settings
            .bind(
                "show-log-timestamps",
                &*imp.renderer_timestamps.get().unwrap(),
                "visible",
            )
            .build();
    }

    pub(crate) fn connect_search_button(&self, search_button: &gtk::ToggleButton) {
        search_button
            .bind_property("active", &*self.imp().search_bar, "search-mode-enabled")
            .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
            .build();
    }

    pub(crate) fn toggle_search(&self) {
        let imp = self.imp();
        imp.search_bar
            .set_search_mode(!imp.search_bar.is_search_mode());
    }

    fn update_search_occurences(&self) {
        let imp = self.imp();

        let search_context = imp.search_context.get().unwrap();
        let count = search_context.occurrences_count();
        imp.search_entry.set_info(&if count > 0 {
            gettext!(
                "{} of {}",
                imp.search_iters
                    .borrow()
                    .as_ref()
                    .map(|(start_iter, end_iter)| search_context
                        .occurrence_position(start_iter, end_iter))
                    .unwrap_or(0),
                count
            )
        } else {
            String::new()
        });
    }

    pub(crate) fn search_backward(&self) {
        let imp = self.imp();

        let iter_at_cursor = imp.source_buffer.iter_at_offset({
            let pos = imp.source_buffer.cursor_position();
            if pos >= 0 {
                pos
            } else {
                i32::MAX
            }
        });

        imp.search_iters.replace_with(|iters| {
            match imp.search_context.get().unwrap().backward(
                &iters
                    .map(|(start_iter, end_iter)| {
                        if iter_at_cursor >= start_iter && iter_at_cursor <= end_iter {
                            start_iter
                        } else {
                            iter_at_cursor
                        }
                    })
                    .unwrap_or(iter_at_cursor),
            ) {
                Some((mut start, end, _)) => {
                    imp.source_view
                        .scroll_to_iter(&mut start, 0.0, false, 0.0, 0.0);
                    imp.source_buffer.place_cursor(&start);

                    Some((start, end))
                }
                None => None,
            }
        });

        self.update_search_occurences();
    }

    pub(crate) fn search_forward(&self) {
        let imp = self.imp();

        let iter_at_cursor = imp.source_buffer.iter_at_offset({
            let pos = imp.source_buffer.cursor_position();
            if pos > 0 {
                pos
            } else {
                0
            }
        });

        imp.search_iters.replace_with(|iters| {
            match imp.search_context.get().unwrap().forward(
                &iters
                    .map(|(start_iter, end_iter)| {
                        if iter_at_cursor >= start_iter && iter_at_cursor <= end_iter {
                            end_iter
                        } else {
                            iter_at_cursor
                        }
                    })
                    .unwrap_or(iter_at_cursor),
            ) {
                Some((start, mut end, _)) => {
                    imp.source_view
                        .scroll_to_iter(&mut end, 0.0, false, 0.0, 0.0);
                    imp.source_buffer.place_cursor(&end);

                    Some((start, end))
                }
                None => None,
            }
        });

        self.update_search_occurences();
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
}

#[derive(Debug, Default)]
pub struct MarkupPerform {
    buffer: String,
    close_tags: Vec<&'static str>,
}

impl MarkupPerform {
    fn move_out_buffer(&mut self) -> String {
        let mut buffer = String::new();
        mem::swap(&mut self.buffer, &mut buffer);
        buffer
    }

    /// Decode the specified bytes. Return true if finished.
    fn decode(&mut self, ansi_encoded_bytes: &[u8]) -> bool {
        let mut parser = vte::Parser::new();

        String::from_utf8_lossy(ansi_encoded_bytes)
            .bytes()
            .for_each(|byte| parser.advance(self, byte));

        self.close_tags.is_empty()
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
            match param {
                [0] => {
                    while let Some(close_tag) = self.close_tags.pop() {
                        self.buffer.push_str(close_tag);
                    }
                }
                items => items
                    .iter()
                    .copied()
                    .filter_map(ansi_escape_to_markup_tags)
                    .for_each(|(start_tag, close_tag)| {
                        self.buffer.push_str(start_tag);
                        self.close_tags.push(close_tag);
                    }),
            }
        }
    }
}

fn ansi_escape_to_markup_tags(item: u16) -> Option<(&'static str, &'static str)> {
    Some(match item {
        1 => ("<b>", "</b>"),
        30 => ("<span foreground=\"#000000\">", "</span>"),
        31 => ("<span foreground=\"#e01b24\">", "</span>"),
        32 => ("<span foreground=\"#33d17a\">", "</span>"),
        33 => ("<span foreground=\"#f6d32d\">", "</span>"),
        34 => ("<span foreground=\"#3584e4\">", "</span>"),
        35 => ("<span foreground=\"#d4267e\">", "</span>"),
        36 => ("<span foreground=\"#00f7f7\">", "</span>"),
        37 => ("<span foreground=\"#ffffff\">", "</span>"),
        _ => return None,
    })
}
