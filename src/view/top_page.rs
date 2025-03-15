use std::cell::OnceCell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use futures::StreamExt;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;

use crate::model;
use crate::podman;
use crate::rt;
use crate::utils;

const ACTION_SEARCH: &str = "top-page.search";
const ACTION_KILL: &str = "top-page.kill";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::TopPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/top_page.ui")]
    pub(crate) struct TopPage {
        pub(super) filter: OnceCell<gtk::Filter>,
        pub(super) search_term: RefCell<String>,
        pub(super) selection: OnceCell<gtk::MultiSelection>,
        pub(super) action_bar: OnceCell<gtk::ActionBar>,
        #[property(get, set, construct_only, nullable)]
        /// A `Container` or a `Pod`
        pub(super) top_source: glib::WeakRef<glib::Object>,
        #[template_child]
        pub(super) toolbar_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub(super) search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) column_view: TemplateChild<gtk::ColumnView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TopPage {
        const NAME: &'static str = "PdsTopPage";
        type Type = super::TopPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.add_binding_action(gdk::Key::F, gdk::ModifierType::CONTROL_MASK, ACTION_SEARCH);

            klass.install_action(ACTION_SEARCH, None, |widget, _, _| {
                widget.toggle_search_mode();
            });

            klass.install_action_async(ACTION_KILL, None, async |widget, _, _| {
                widget.kill_selected_processes().await;
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TopPage {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            self.search_entry.set_key_capture_widget(Some(obj));

            if let Some(top_source) = obj.top_source() {
                if let Some(container) = top_source.downcast_ref::<model::Container>() {
                    self.window_title.set_title(&gettext("Container Processes"));
                    container.property_expression_weak("name").bind(
                        &*self.window_title,
                        "subtitle",
                        glib::Object::NONE,
                    );
                } else if let Some(pod) = top_source.downcast_ref::<model::Pod>() {
                    self.window_title.set_title(&gettext("Pod Processes"));
                    self.window_title.set_subtitle(&pod.name());
                }
            }

            #[derive(Clone, Copy)]
            enum PropertyType {
                String,
                Integer,
                Float,
                Elapsed,
                CpuTime,
            }

            [
                (
                    "user",
                    PropertyType::String,
                    gettext("USER"),
                    false,
                    gtk::Align::Start,
                ),
                (
                    "pid",
                    PropertyType::Integer,
                    gettext("PID"),
                    false,
                    gtk::Align::End,
                ),
                (
                    "ppid",
                    PropertyType::Integer,
                    gettext("PPID"),
                    false,
                    gtk::Align::End,
                ),
                (
                    "cpu",
                    PropertyType::Float,
                    gettext("%CPU"),
                    false,
                    gtk::Align::End,
                ),
                (
                    "elapsed",
                    PropertyType::Elapsed,
                    gettext("ELAPSED"),
                    false,
                    gtk::Align::End,
                ),
                (
                    "tty",
                    PropertyType::String,
                    gettext("TTY"),
                    false,
                    gtk::Align::Start,
                ),
                (
                    "time",
                    PropertyType::CpuTime,
                    gettext("TIME"),
                    false,
                    gtk::Align::End,
                ),
                (
                    "command",
                    PropertyType::String,
                    gettext("COMMAND"),
                    true,
                    gtk::Align::Start,
                ),
            ]
            .into_iter()
            .for_each(|(property_name, property_type, title, hexpand, halign)| {
                let property_expr = model::Process::this_expression(property_name);
                let display_expr: gtk::Expression = match property_type {
                    PropertyType::String | PropertyType::Integer => property_expr.clone().upcast(),
                    PropertyType::Float => property_expr
                        .chain_closure::<String>(closure!(|_: glib::Object, float: f64| {
                            format!(
                                "{:.1$}",
                                float,
                                if float >= 100.0 {
                                    0
                                } else if float >= 10.0 {
                                    1
                                } else {
                                    2
                                }
                            )
                        }))
                        .upcast(),
                    PropertyType::Elapsed => property_expr
                        .chain_closure::<String>(closure!(|_: glib::Object, elapsed: u64| {
                            format_elapsed(elapsed as i64)
                        }))
                        .upcast(),
                    PropertyType::CpuTime => property_expr
                        .chain_closure::<String>(closure!(|_: glib::Object, cpu_time: u64| {
                            format_cpu_time(cpu_time as i64)
                        }))
                        .upcast(),
                };

                let expr_watches = Rc::new(RefCell::new(HashMap::new()));

                let factory = gtk::SignalListItemFactory::new();
                factory.connect_setup(clone!(
                    #[weak]
                    expr_watches,
                    move |_, list_item| {
                        let label = gtk::Label::builder().halign(halign).build();
                        if matches!(
                            property_type,
                            PropertyType::Integer
                                | PropertyType::Float
                                | PropertyType::Elapsed
                                | PropertyType::CpuTime
                        ) {
                            label.add_css_class("numeric");
                        }
                        list_item
                            .downcast_ref::<gtk::ListItem>()
                            .unwrap()
                            .set_child(Some(&label));

                        expr_watches.borrow_mut().insert(label, None);
                    }
                ));
                factory.connect_bind(clone!(
                    #[strong]
                    display_expr,
                    #[strong]
                    expr_watches,
                    move |_, list_item| {
                        let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();

                        let process = list_item
                            .item()
                            .unwrap()
                            .downcast::<model::Process>()
                            .unwrap();

                        let label = list_item.child().and_downcast::<gtk::Label>().unwrap();
                        let expr_watch = display_expr.bind(&label, "label", Some(&process));
                        *expr_watches.borrow_mut().get_mut(&label).unwrap() = Some(expr_watch);
                    }
                ));

                factory.connect_unbind(clone!(
                    #[weak]
                    expr_watches,
                    move |_, list_item| {
                        let label = list_item
                            .downcast_ref::<gtk::ListItem>()
                            .unwrap()
                            .child()
                            .and_downcast::<gtk::Label>()
                            .unwrap();
                        expr_watches
                            .borrow()
                            .get(&label)
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .unwatch();
                    }
                ));

                factory.connect_teardown(move |_, list_item| {
                    let label = list_item
                        .downcast_ref::<gtk::ListItem>()
                        .unwrap()
                        .child()
                        .and_downcast::<gtk::Label>()
                        .unwrap();
                    expr_watches.borrow_mut().remove(&label);
                });

                let column = gtk::ColumnViewColumn::builder()
                    .title(title)
                    .factory(&factory)
                    .expand(hexpand)
                    .sorter(&match property_type {
                        PropertyType::String => {
                            gtk::StringSorter::new(Some(property_expr)).upcast::<gtk::Sorter>()
                        }
                        PropertyType::Integer
                        | PropertyType::Float
                        | PropertyType::Elapsed
                        | PropertyType::CpuTime => {
                            gtk::NumericSorter::new(Some(property_expr)).upcast::<gtk::Sorter>()
                        }
                    })
                    .build();

                self.column_view.append_column(&column);
            });

            let model = obj
                .top_source()
                .map(|top_source| {
                    if let Some(container) = top_source.downcast_ref::<model::Container>() {
                        model::ProcessList::from(container)
                    } else if let Some(pod) = top_source.downcast_ref::<model::Pod>() {
                        model::ProcessList::from(pod)
                    } else {
                        unreachable!()
                    }
                })
                .unwrap();

            let sorter = self.column_view.sorter().unwrap();
            model.connect_updated(clone!(
                #[weak]
                sorter,
                move |_| {
                    sorter.changed(gtk::SorterChange::Different);
                }
            ));

            let filter = gtk::CustomFilter::new(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                false,
                move |item| {
                    let term = &*obj.imp().search_term.borrow();

                    if term.is_empty() {
                        true
                    } else {
                        let process = item.downcast_ref::<model::Process>().unwrap();
                        process.user().to_lowercase().contains(term)
                            || process.tty().to_lowercase().contains(term)
                            || process.command().to_lowercase().contains(term)
                            || process.pid().to_string().contains(term)
                            || process.ppid().to_string().contains(term)
                    }
                }
            ));
            let filter_list_model = gtk::FilterListModel::new(Some(model), Some(filter.clone()));
            self.filter.set(filter.upcast()).unwrap();

            let selection_model = gtk::MultiSelection::new(Some(gtk::SortListModel::new(
                Some(filter_list_model),
                Some(sorter),
            )));
            match obj.top_source().and_downcast_ref::<model::Container>() {
                Some(_) => {
                    let action_bar = gtk::Builder::from_resource(
                        "/com/github/marhkb/Pods/ui/view/top_page_action_bar.ui",
                    )
                    .object::<gtk::ActionBar>("action_bar")
                    .unwrap();

                    action_bar.set_revealed(false);
                    self.toolbar_view.add_bottom_bar(&action_bar);

                    selection_model.connect_items_changed(clone!(
                        #[weak]
                        action_bar,
                        move |model, _, removed, _| {
                            if removed > 0 {
                                action_bar.set_revealed(model.selection().size() > 0);
                            }
                        }
                    ));
                    selection_model.connect_selection_changed(clone!(
                        #[weak]
                        action_bar,
                        move |model, position, _| {
                            action_bar.set_revealed(
                                model.is_selected(position) || model.selection().size() > 0,
                            );
                        }
                    ));

                    self.action_bar.set(action_bar).unwrap();
                }
                None => obj.action_set_enabled(ACTION_KILL, false),
            }

            self.column_view.set_model(Some(&selection_model));

            self.selection.set(selection_model).unwrap();
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for TopPage {}

    #[gtk::template_callbacks]
    impl TopPage {
        #[template_callback]
        fn on_notify_search_mode_enabled(&self) {
            if self.search_bar.is_search_mode() {
                self.search_entry.grab_focus();
            } else {
                self.search_entry.set_text("");
            }
        }

        #[template_callback]
        fn on_search_started(&self) {
            self.search_button.set_active(true)
        }

        #[template_callback]
        fn on_search_changed(&self) {
            let term = self.search_entry.text().trim().to_lowercase();

            let filter_change = if self.search_term.borrow().contains(&term) {
                gtk::FilterChange::LessStrict
            } else {
                gtk::FilterChange::MoreStrict
            };

            self.search_term.replace(term);
            self.update_filter(filter_change);
        }

        fn update_filter(&self, filter_change: gtk::FilterChange) {
            if let Some(filter) = self.filter.get() {
                filter.changed(filter_change);
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct TopPage(ObjectSubclass<imp::TopPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for TopPage {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder()
            .property("top-source", container)
            .build()
    }
}

impl From<&model::Pod> for TopPage {
    fn from(pod: &model::Pod) -> Self {
        glib::Object::builder().property("top-source", pod).build()
    }
}

impl TopPage {
    pub(crate) fn set_search_mode(&self, value: bool) {
        self.imp().search_bar.set_search_mode(value);
    }

    pub(crate) fn toggle_search_mode(&self) {
        self.set_search_mode(!self.imp().search_bar.is_search_mode());
    }

    pub(crate) async fn kill_selected_processes(&self) {
        let top_source = self.top_source();

        let selection = self.imp().selection.get().unwrap();
        let selected_positions = selection.selection();

        if let Some((iter, first_pid)) = gtk::BitsetIter::init_first(&selected_positions) {
            let mut selected_pids = iter.chain(Some(first_pid)).collect::<Vec<_>>();
            selected_pids.sort_unstable_by(|lhs, rhs| rhs.cmp(lhs));

            let selected_pids = selected_pids
                .into_iter()
                .map(|position| {
                    selection
                        .item(position)
                        .and_downcast::<model::Process>()
                        .unwrap()
                        .pid()
                        .to_string()
                })
                .collect::<Vec<_>>();

            match top_source.and_downcast_ref::<model::Container>() {
                Some(container) => {
                    let result = rt::Promise::new({
                        let container = container.api().unwrap();
                        let opts = podman::opts::ExecCreateOpts::builder()
                            .attach_stderr(true)
                            .attach_stdout(false)
                            .attach_stdin(false)
                            .tty(false)
                            .command(
                                ["kill", "-9"]
                                    .into_iter()
                                    .chain(selected_pids.iter().map(String::as_str)),
                            )
                            .build();
                        async move {
                            let exec = container.create_exec(&opts).await.unwrap();

                            let opts = podman::opts::ExecStartOpts::builder().tty(false).build();
                            let (reader, _) = exec.start(&opts).await.unwrap().unwrap().split();

                            let err_output = reader
                                .map(Result::unwrap)
                                .map(Vec::from)
                                .map(String::from_utf8)
                                .map(Result::unwrap)
                                .collect::<String>()
                                .await;

                            if err_output.is_empty() {
                                Ok(())
                            } else {
                                Err(err_output)
                            }
                        }
                    })
                    .exec()
                    .await;

                    if let Err(e) = result {
                        utils::show_error_toast(self, &gettext("Error"), e.trim());
                    }
                }
                None => utils::show_error_toast(
                    self,
                    &gettext("Error"),
                    &gettext("Killing pod processes is not supported"),
                ),
            }
        }
    }
}

pub(crate) fn format_elapsed(mut millis: i64) -> String {
    let hours = millis / (60 * 60 * 1_000);
    millis %= 60 * 60 * 1_000;
    let minutes = millis / (60 * 1_000);
    millis %= 60 * 1_000;
    let seconds = millis / 1_000;

    if hours > 0 {
        format!("{}h{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}.{:02}", minutes, seconds, (millis % 1_000) / 100)
    }
}

pub(crate) fn format_cpu_time(mut millis: i64) -> String {
    let minutes = millis / (60 * 1_000);
    millis %= 60 * 1_000;
    let seconds = millis / 1_000;
    millis = (millis % 1_000) / 100;

    format!("{}:{:02}.{:02}", minutes, seconds, millis)
}
