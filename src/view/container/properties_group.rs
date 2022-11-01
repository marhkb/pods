use adw::subclass::prelude::PreferencesGroupImpl;
use adw::traits::ExpanderRowExt;
use gettextrs::gettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::pango;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_RENAME: &str = "container-properties-group.rename";
const ACTION_SHOW_HEALTH_DETAILS: &str = "container-properties-group.show-health-details";
const ACTION_SHOW_IMAGE_DETAILS: &str = "container-properties-group.show-image-details";
const ACTION_SHOW_POD_DETAILS: &str = "container-properties-group.show-pod-details";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container/properties-group.ui")]
    pub(crate) struct PropertiesGroup {
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) inspection_spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) id_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) created_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) state_since_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) port_bindings_row: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub(super) port_bindings_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) health_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) health_status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) image_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) image_action_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) pod_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) pod_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PropertiesGroup {
        const NAME: &'static str = "PdsContainerPropertiesGroup";
        type Type = super::PropertiesGroup;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_RENAME, None, move |widget, _, _| {
                widget.rename();
            });
            klass.install_action(ACTION_SHOW_HEALTH_DETAILS, None, move |widget, _, _| {
                widget.show_health_details();
            });
            klass.install_action(ACTION_SHOW_IMAGE_DETAILS, None, move |widget, _, _| {
                widget.show_image_details();
            });
            klass.install_action(ACTION_SHOW_POD_DETAILS, None, move |widget, _, _| {
                widget.show_pod_details();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PropertiesGroup {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Container>("container")
                        .flags(
                            glib::ParamFlags::READWRITE
                                | glib::ParamFlags::CONSTRUCT
                                | glib::ParamFlags::EXPLICIT_NOTIFY,
                        )
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "container" => self.instance().set_container(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container" => self.instance().container().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            let ticks_expr = Self::Type::this_expression("root")
                .chain_property::<gtk::Window>("application")
                .chain_property::<crate::Application>("ticks");
            let container_expr = Self::Type::this_expression("container");
            let data_expr = container_expr.chain_property::<model::Container>("data");
            let status_expr = container_expr.chain_property::<model::Container>("status");
            let health_status_expr =
                container_expr.chain_property::<model::Container>("health_status");
            let port_bindings_expr =
                data_expr.chain_property::<model::ContainerData>("port-bindings");
            let image_expr = container_expr.chain_property::<model::Container>("image");
            let pod_expr = container_expr.chain_property::<model::Container>("pod");

            data_expr
                .chain_closure::<bool>(closure!(
                    |_: Self::Type, cmd: Option<model::ContainerData>| { cmd.is_none() }
                ))
                .bind(&*self.inspection_spinner, "visible", Some(obj));

            container_expr
                .chain_property::<model::Container>("id")
                .chain_closure::<String>(closure!(|_: Self::Type, id: &str| {
                    id.chars().take(12).collect::<String>()
                }))
                .bind(&*self.id_row, "value", Some(obj));

            gtk::ClosureExpression::new::<String>(
                &[
                    &ticks_expr,
                    &container_expr.chain_property::<model::Container>("created"),
                ],
                closure!(|_: Self::Type, _ticks: u64, created: i64| {
                    utils::format_ago(utils::timespan_now(created))
                }),
            )
            .bind(&*self.created_row, "value", Some(obj));

            port_bindings_expr.watch(
                Some(obj),
                clone!(@weak obj, @to-owned port_bindings_expr => move || {
                    let imp = obj.imp();

                    imp.port_bindings_label.set_label("");
                    utils::ChildIter::from(&*imp.port_bindings_row)
                        .for_each(|child| imp.port_bindings_row.remove(&child));

                    let port_bindings: Option<model::BoxedPortBindings> =
                        port_bindings_expr.evaluate_as(Some(&obj));

                    if let Some(port_bindings) = port_bindings {
                        let client = obj
                            .container()
                            .unwrap()
                            .container_list()
                            .unwrap()
                            .client()
                            .unwrap();
                        let connection = client.connection();

                        imp.port_bindings_label.set_label(&port_bindings.len().to_string());

                        port_bindings
                            .iter()
                            .flat_map(|(container_port, hosts)| {
                                hosts.iter().map(move |host| (container_port, host))
                            })
                            .map(|(container_port, host)| {
                                let host_ip = host.host_ip.as_deref().unwrap_or("");
                                let host = format!(
                                    "{}:{}",
                                    if host_ip.is_empty() {
                                        if connection.is_remote() {
                                            connection.url()
                                        } else {
                                            "127.0.0.1"
                                        }
                                    } else {
                                        host_ip
                                    },
                                    host.host_port.as_deref().unwrap_or("0")
                                );

                                let box_ = gtk::CenterBox::builder()
                                    .margin_top(12)
                                    .margin_end(12)
                                    .margin_bottom(12)
                                    .margin_start(12)
                                    .build();

                                box_.set_start_widget(Some(
                                    &gtk::Label::builder()
                                        .label(&format!("<a href='http://{}'>{}</a>", host, host))
                                        .hexpand(true)
                                        .selectable(true)
                                        .use_markup(true)
                                        .wrap(true)
                                        .wrap_mode(pango::WrapMode::WordChar)
                                        .xalign(0.0)
                                        .build(),
                                ));
                                box_.set_center_widget(Some(
                                    &gtk::Image::builder()
                                        .icon_name("arrow1-right-symbolic")
                                        .margin_start(15)
                                        .margin_end(12)
                                        .build()
                                ));
                                box_.set_end_widget(Some(&gtk::Label::builder()
                                    .label(container_port)
                                    .css_classes(vec!["dim-label".to_string()])
                                    .hexpand(true)
                                    .selectable(true)
                                    .wrap(true)
                                    .wrap_mode(pango::WrapMode::WordChar)
                                    .xalign(1.0)
                                    .build()
                                ));

                                gtk::ListBoxRow::builder()
                                    .activatable(false)
                                    .child(&box_)
                                    .build()
                            })
                            .for_each(|row| imp.port_bindings_row.add_row(&row));
                    }
                }),
            );

            #[rustfmt::skip]
            port_bindings_expr.chain_closure::<bool>(
                closure!(|_: Self::Type, port_bindings: Option<&model::BoxedPortBindings>| {
                    port_bindings.map(|map| !map.is_empty()).unwrap_or(false)
                }))
                .bind(&*self.port_bindings_row, "visible", Some(obj));

            gtk::ClosureExpression::new::<String>(
                &[
                    &ticks_expr,
                    &status_expr,
                    &container_expr.chain_property::<model::Container>("up-since"),
                ],
                closure!(|_: Self::Type,
                          _ticks: u64,
                          status: model::ContainerStatus,
                          up_since: i64| {
                    use model::ContainerStatus::*;

                    match status {
                        Running | Paused => {
                            // Translators: Example: since {3 hours}, since {a few seconds}
                            gettext!(
                                "since {}",
                                utils::human_friendly_timespan(utils::timespan_now(up_since))
                            )
                        }
                        _ => String::new(),
                    }
                }),
            )
            .bind(&*self.state_since_label, "label", Some(obj));

            status_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| status.to_string()
                ))
                .bind(&*self.status_label, "label", Some(obj));

            let css_classes = self.status_label.css_classes();
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(glib::GString::from(
                                super::super::container_status_css_class(status),
                            )))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.status_label, "css-classes", Some(obj));

            health_status_expr.watch(
                Some(obj),
                clone!(@weak obj => move || {
                    obj.action_set_enabled(
                        ACTION_SHOW_HEALTH_DETAILS,
                        obj.container()
                            .as_ref()
                            .map(model::Container::health_status)
                            .map(|status| status != model::ContainerHealthStatus::Unconfigured)
                            .unwrap_or(false),
                    );
                }),
            );

            health_status_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, status: model::ContainerHealthStatus| status.to_string()
                ))
                .bind(&*self.health_status_label, "label", Some(obj));

            let css_classes = self.status_label.css_classes();
            health_status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::ContainerHealthStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(glib::GString::from(
                                super::super::container_health_status_css_class(status),
                            )))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.health_status_label, "css-classes", Some(obj));

            gtk::ClosureExpression::new::<String>(
                &[
                    image_expr.chain_property::<model::Image>("repo-tags"),
                    image_expr.chain_property::<model::Image>("id"),
                ],
                closure!(|_: Self::Type, repo_tags: gtk::StringList, id: &str| {
                    repo_tags
                        .string(0)
                        .map(String::from)
                        .unwrap_or_else(|| id.chars().take(12).collect())
                }),
            )
            .bind(&*self.image_label, "label", Some(obj));

            image_expr.watch(
                Some(obj),
                clone!(@weak obj => move || {
                    obj.action_set_enabled(
                        ACTION_SHOW_IMAGE_DETAILS,
                        obj.container().as_ref().and_then(model::Container::image).is_some()
                    );
                }),
            );

            image_expr
                .chain_closure::<String>(closure!(|_: Self::Type, image: Option<model::Image>| {
                    match image {
                        None => "waiting",
                        Some(_) => "ready",
                    }
                }))
                .bind(&*self.image_action_stack, "visible-child-name", Some(obj));

            pod_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, pod: Option<model::Pod>| {
                    pod.is_some()
                }))
                .bind(&*self.pod_row, "visible", Some(obj));

            pod_expr
                .chain_closure::<String>(closure!(|_: Self::Type, pod: Option<model::Pod>| {
                    pod.as_ref().map(model::Pod::name).unwrap_or_default()
                }))
                .bind(&*self.pod_label, "label", Some(obj));
        }
    }

    impl WidgetImpl for PropertiesGroup {}
    impl PreferencesGroupImpl for PropertiesGroup {}
}

glib::wrapper! {
    pub(crate) struct PropertiesGroup(ObjectSubclass<imp::PropertiesGroup>)
        @extends gtk::Widget, adw::PreferencesGroup,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PropertiesGroup {
    pub(crate) fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    pub(crate) fn set_container(&self, value: Option<&model::Container>) {
        if self.container().as_ref() == value {
            return;
        }
        self.imp().container.set(value);
        self.notify("container");
    }

    fn rename(&self) {
        let dialog = view::ContainerRenameDialog::from(self.container());
        dialog.set_transient_for(Some(&utils::root(self)));
        dialog.present();
    }

    fn show_health_details(&self) {
        if let Some(ref container) = self.container() {
            self.show_details(&view::ContainerHealthCheckPage::from(container));
        }
    }

    fn show_image_details(&self) {
        self.show_details(&view::ImageDetailsPage::from(
            &self.container().unwrap().image().unwrap(),
        ));
    }

    fn show_pod_details(&self) {
        if let Some(pod) = self.container().as_ref().and_then(model::Container::pod) {
            self.show_details(&view::PodDetailsPage::from(&pod));
        }
    }

    fn show_details<W: glib::IsA<gtk::Widget>>(&self, widget: &W) {
        utils::find_leaflet_overlay(self).show_details(widget);
    }
}
