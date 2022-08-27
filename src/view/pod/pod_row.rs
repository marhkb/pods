use adw::subclass::prelude::ActionRowImpl;
use adw::subclass::prelude::PreferencesRowImpl;
use gtk::glib;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/pod-row.ui")]
    pub(crate) struct PodRow {
        pub(super) pod: WeakRef<model::Pod>,
        #[template_child]
        pub(super) stats_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) cpu_bar: TemplateChild<view::CircularProgressBar>,
        #[template_child]
        pub(super) mem_bar: TemplateChild<view::CircularProgressBar>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodRow {
        const NAME: &'static str = "PodRow";
        type Type = super::PodRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("pod.show-details", None, move |widget, _, _| {
                widget.show_details();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "pod",
                    "Pod",
                    "The pod of this PodRow",
                    model::Pod::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                )]
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
                "pod" => obj.set_pod(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "pod" => obj.pod().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let pod_expr = Self::Type::this_expression("pod");
            let stats_expr = pod_expr.chain_property::<model::Pod>("stats");
            let status_expr = pod_expr.chain_property::<model::Pod>("status");

            pod_expr
                .chain_property::<model::Pod>("name")
                .chain_closure::<String>(closure!(|_: glib::Object, name: Option<String>| {
                    utils::escape(&utils::format_option(name))
                }))
                .bind(obj, "title", Some(obj));

            pod_expr
                .chain_property::<model::Pod>("id")
                .chain_closure::<String>(closure!(|_: glib::Object, id: &str| {
                    id.chars().take(12).collect::<String>()
                }))
                .bind(obj, "subtitle", Some(obj));

            status_expr
                .chain_closure::<bool>(closure!(
                    |_: glib::Object, status: model::PodStatus| matches!(
                        status,
                        model::PodStatus::Running
                    )
                ))
                .bind(&*self.stats_box, "visible", Some(obj));

            stats_expr
                .chain_closure::<f64>(closure!(
                    |_: glib::Object, stats: Option<model::BoxedPodStats>| {
                        println!("{stats:?}");
                        stats
                            .and_then(|stats| {
                                stats.cpu.as_ref().map(|perc| {
                                    perc.split_at(perc.len() - 2).0.parse::<f64>().unwrap() * 1.0
                                })
                            })
                            .unwrap_or_default()
                    }
                ))
                .bind(&*self.cpu_bar, "percentage", Some(obj));

            // stats_expr
            //     .chain_closure::<f64>(closure!(
            //         |_: glib::Object, stats: Option<model::BoxedPodStats>| {
            //             stats
            //                 .and_then(|stats| {
            //                     stats
            //                         .mem
            //                         .as_ref()
            //                         .map(|perc| perc.parse::<f64>().unwrap() * 0.01)
            //                 })
            //                 .unwrap_or_default()
            //         }
            //     ))
            //     .bind(&*self.mem_bar, "percentage", Some(obj));

            status_expr
                .chain_closure::<String>(closure!(|_: glib::Object, status: model::PodStatus| {
                    status.to_string()
                }))
                .bind(&*self.status_label, "label", Some(obj));

            let css_classes = self.status_label.css_classes();
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: glib::Object, status: model::PodStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(glib::GString::from(
                                super::super::pod_status_css_class(status),
                            )))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.status_label, "css-classes", Some(obj));
        }
    }

    impl WidgetImpl for PodRow {}
    impl ListBoxRowImpl for PodRow {}
    impl PreferencesRowImpl for PodRow {}
    impl ActionRowImpl for PodRow {}
}

glib::wrapper! {
    pub(crate) struct PodRow(ObjectSubclass<imp::PodRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

impl From<&model::Pod> for PodRow {
    fn from(pod: &model::Pod) -> Self {
        glib::Object::new(&[("pod", pod)]).expect("Failed to create PodRow")
    }
}

impl PodRow {
    pub(crate) fn pod(&self) -> Option<model::Pod> {
        self.imp().pod.upgrade()
    }

    fn set_pod(&self, value: Option<&model::Pod>) {
        if self.pod().as_ref() == value {
            return;
        }
        self.imp().pod.set(value);
        self.notify("pod");
    }

    fn show_details(&self) {
        utils::find_leaflet_overlay(self)
            .show_details(&view::PodDetailsPage::from(&self.pod().unwrap()));
    }
}
