use std::cell::OnceCell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

const ACTION_SEARCH: &str = "top-page.search";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::TopPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/top_page.ui")]
    pub(crate) struct TopPage {
        pub(super) filter: OnceCell<gtk::Filter>,
        pub(super) search_term: RefCell<String>,
        #[property(get, set, construct_only, nullable)]
        /// A `Container` or a `Pod`
        pub(super) top_source: glib::WeakRef<glib::Object>,
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

            klass.add_binding_action(
                gdk::Key::F,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_SEARCH,
                None,
            );

            klass.install_action(ACTION_SEARCH, None, |widget, _, _| {
                widget.toggle_search_mode();
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
                factory.connect_setup(clone!(@weak expr_watches => move |_, list_item| {
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
                }));
                factory.connect_bind(
                    clone!(@strong display_expr, @strong expr_watches => move |_, list_item| {
                        let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();

                        let process = list_item
                            .item()
                            .unwrap()
                            .downcast::<model::Process>()
                            .unwrap();

                        let label = list_item.child().and_downcast::<gtk::Label>().unwrap();
                        let expr_watch = display_expr.bind(&label, "label", Some(&process));
                        *expr_watches.borrow_mut().get_mut(&label).unwrap() = Some(expr_watch);
                    }),
                );

                factory.connect_unbind(clone!(@weak expr_watches => move |_, list_item| {
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
                }));

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
            model.connect_updated(clone!(@weak sorter => move |_| {
                sorter.changed(gtk::SorterChange::Different);
            }));

            let filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    let term = &*obj.imp().search_term.borrow();

                    if term.is_empty() {
                        true
                    } else {
                        let process = item.downcast_ref::<model::Process>().unwrap();
                        process
                            .user().to_lowercase().contains(term)
                            || process
                                .tty().to_lowercase().contains(term)
                            || process
                                .command().to_lowercase().contains(term)
                            || process.pid().to_string().contains(term)
                            || process.ppid().to_string().contains(term)
                    }
                }));
            let filter_list_model = gtk::FilterListModel::new(Some(model), Some(filter.clone()));
            self.filter.set(filter.upcast()).unwrap();

            self.column_view
                .set_model(Some(&gtk::MultiSelection::new(Some(
                    gtk::SortListModel::new(Some(filter_list_model), Some(sorter)),
                ))));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
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
