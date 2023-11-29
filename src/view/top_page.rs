use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::TopPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/top_page.ui")]
    pub(crate) struct TopPage {
        #[property(get, set, construct_only, nullable)]
        /// A `Container` or a `Pod`
        pub(super) top_source: glib::WeakRef<glib::Object>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
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

            self.column_view
                .set_model(Some(&gtk::MultiSelection::new(Some(
                    gtk::SortListModel::new(Some(model), Some(sorter)),
                ))));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for TopPage {}
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
