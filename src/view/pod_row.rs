use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::property::PropertySet;
use glib::Properties;
use gtk::glib;
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
    #[template(resource = "/com/github/marhkb/Pods/ui/view/pod_row.ui")]
    pub(crate) struct PodRow {
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[property(get, set, construct, nullable)]
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
        pub(super) ports_flow_box: TemplateChild<gtk::FlowBox>,
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
            klass.bind_template_callbacks();

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
                        model::PodStatus::Degraded => "heart-broken-symbolic",
                        _ => "media-playback-stop-symbolic",
                    }
                }))
                .bind(&*self.spinner, "icon-name", Some(obj));

            let css_classes = utils::css_classes(&*self.spinner);
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

    #[gtk::template_callbacks]
    impl PodRow {
        #[template_callback]
        fn on_notify_pod(&self) {
            let mut bindings = self.bindings.borrow_mut();
            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }

            let obj = &*self.obj();

            if let Some(pod) = obj.pod() {
                let binding = pod
                    .bind_property("selected", &*self.check_button, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();

                bindings.push(binding);

                match pod.infra_container() {
                    Some(container) => self.setup_ports(&container),
                    None => {
                        let handler_id_ref = Rc::new(RefCell::new(None));
                        let handler_id = pod.connect_infra_container_notify(clone!(
                            #[weak]
                            obj,
                            #[strong]
                            handler_id_ref,
                            move |pod| {
                                if let Some(container) = pod.infra_container() {
                                    pod.disconnect(handler_id_ref.take().unwrap());
                                    obj.imp().setup_ports(&container);
                                }
                            }
                        ));
                        handler_id_ref.set(Some(handler_id));
                    }
                }
            }
        }

        fn setup_ports(&self, container: &model::Container) {
            let obj = &self.obj();

            model::Container::this_expression("ports")
                .chain_property::<model::PortMappingList>("len")
                .chain_closure::<bool>(closure!(|_: model::Container, len: u32| { len > 0 }))
                .bind(&self.ports_flow_box.get(), "visible", Some(container));

            self.ports_flow_box.bind_model(
                Some(&container.ports()),
                clone!(
                    #[weak]
                    obj,
                    #[upgrade_or_panic]
                    move |item| {
                        let port_mapping = item.downcast_ref::<model::PortMapping>().unwrap();

                        let label = gtk::Label::builder()
                            .css_classes(["status-badge-small", "numeric"])
                            .halign(gtk::Align::Center)
                            .label(format!(
                                "{}/{}",
                                port_mapping.host_port(),
                                port_mapping.protocol()
                            ))
                            .build();

                        let css_classes = utils::css_classes(&label);
                        super::PodRow::this_expression("pod")
                            .chain_property::<model::Pod>("status")
                            .chain_closure::<Vec<String>>(closure!(
                                |_: super::PodRow, status: model::PodStatus| {
                                    css_classes
                                        .iter()
                                        .cloned()
                                        .chain(Some(String::from(
                                            super::super::pod_status_css_class(status),
                                        )))
                                        .collect::<Vec<_>>()
                                }
                            ))
                            .bind(&label, "css-classes", Some(&obj));

                        gtk::FlowBoxChild::builder()
                            .halign(gtk::Align::Start)
                            .child(&label)
                            .build()
                            .upcast()
                    }
                ),
            );
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
                utils::navigation_view(self).push(
                    &adw::NavigationPage::builder()
                        .title(gettext("Pod Details"))
                        .child(&view::PodDetailsPage::from(pod))
                        .build(),
                );
            }
        }
    }
}
