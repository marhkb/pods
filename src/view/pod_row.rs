use std::cell::RefCell;

use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::model::SelectableExt;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;
use crate::widget;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PodRow)]
    #[template(file = "pod_row.ui")]
    pub(crate) struct PodRow {
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[property(get, set = Self::set_pod, construct, nullable)]
        pub(super) pod: glib::WeakRef<model::Pod>,
        #[template_child]
        pub(super) spinner: TemplateChild<widget::Spinner>,
        #[template_child]
        pub(super) check_button_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) id_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) end_box_revealer: TemplateChild<gtk::Revealer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodRow {
        const NAME: &'static str = "PdsPodRow";
        type Type = super::PodRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("pod-row.activate", None, |widget, _, _| {
                widget.activate();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodRow {
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

            let pod_expr = Self::Type::this_expression("pod");

            let selection_mode_expr = pod_expr
                .chain_property::<model::Pod>("pod-list")
                .chain_property::<model::PodList>("selection-mode");

            selection_mode_expr.bind(&*self.check_button_revealer, "reveal-child", Some(obj));
            selection_mode_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, is_selection_mode: bool| {
                    !is_selection_mode
                }))
                .bind(&*self.end_box_revealer, "reveal-child", Some(obj));

            let status_expr = pod_expr.chain_property::<model::Pod>("status");

            pod_expr
                .chain_property::<model::Pod>("action-ongoing")
                .bind(&*self.spinner, "spinning", Some(obj));

            status_expr
                .chain_closure::<String>(closure!(|_: Self::Type, status: model::PodStatus| {
                    match status {
                        model::PodStatus::Running => "media-playback-start-symbolic",
                        model::PodStatus::Paused => "media-playback-pause-symbolic",
                        model::PodStatus::Degraded => "degraded-pod-symbolic",
                        _ => "media-playback-stop-symbolic",
                    }
                }))
                .bind(&*self.spinner, "icon-name", Some(obj));

            let css_classes = utils::css_classes(self.spinner.upcast_ref());
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::PodStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(String::from(view::pod_status_css_class(status))))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.spinner, "css-classes", Some(obj));

            gtk::ClosureExpression::new::<String>(
                &[
                    pod_expr.chain_property::<model::Pod>("name"),
                    pod_expr.chain_property::<model::Pod>("to-be-deleted"),
                ],
                closure!(|_: Self::Type, name: &str, to_be_deleted: bool| {
                    let title = utils::escape(name);
                    if to_be_deleted {
                        format!("<s>{title}</s>")
                    } else {
                        title
                    }
                }),
            )
            .bind(&*self.name_label, "label", Some(obj));

            pod_expr
                .chain_property::<model::Pod>("id")
                .chain_closure::<String>(closure!(|_: Self::Type, id: &str| utils::format_id(id)))
                .bind(&*self.id_label, "label", Some(obj));
        }
    }

    impl WidgetImpl for PodRow {}
    impl ListBoxRowImpl for PodRow {}

    impl PodRow {
        pub(super) fn set_pod(&self, value: Option<&model::Pod>) {
            let obj = &*self.obj();
            if obj.pod().as_ref() == value {
                return;
            }

            let mut bindings = self.bindings.borrow_mut();
            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }

            if let Some(pod) = value {
                let binding = pod
                    .bind_property("selected", &*self.check_button, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();

                bindings.push(binding);
            }

            self.pod.set(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct PodRow(ObjectSubclass<imp::PodRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::Actionable, gtk::ConstraintTarget;
}

impl From<&model::Pod> for PodRow {
    fn from(pod: &model::Pod) -> Self {
        glib::Object::builder().property("pod", pod).build()
    }
}

impl PodRow {
    pub(crate) fn activate(&self) {
        if let Some(pod) = self.pod().as_ref() {
            if pod
                .pod_list()
                .map(|list| list.is_selection_mode())
                .unwrap_or(false)
            {
                pod.select();
            } else {
                utils::find_leaflet_overlay(self.upcast_ref())
                    .show_details(view::PodDetailsPage::from(pod).upcast_ref());
            }
        }
    }
}
