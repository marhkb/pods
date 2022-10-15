use adw::subclass::prelude::PreferencesGroupImpl;
use gettextrs::gettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
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
        pub(super) id_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) port_bindings_row: TemplateChild<view::PropertyWidgetRow>,
        #[template_child]
        pub(super) port_bindings_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) created_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) state_since_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
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
                vec![glib::ParamSpecObject::new(
                    "container",
                    "Container",
                    "The container of properties group",
                    model::Container::static_type(),
                    glib::ParamFlags::READWRITE
                        | glib::ParamFlags::CONSTRUCT
                        | glib::ParamFlags::EXPLICIT_NOTIFY,
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
                "container" => obj.set_container(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container" => obj.container().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let container_expr = Self::Type::this_expression("container");
            let status_expr = container_expr.chain_property::<model::Container>("status");
            let health_status_expr =
                container_expr.chain_property::<model::Container>("health_status");
            let port_bindings_expr =
                container_expr.chain_property::<model::Container>("port-bindings");
            let image_expr = container_expr.chain_property::<model::Container>("image");
            let pod_expr = container_expr.chain_property::<model::Container>("pod");

            container_expr
                .chain_property::<model::Container>("id")
                .chain_closure::<String>(closure!(|_: glib::Object, id: &str| {
                    id.chars().take(12).collect::<String>()
                }))
                .bind(&*self.id_row, "value", Some(obj));

            container_expr
                .chain_property::<model::Container>("created")
                .chain_closure::<String>(closure!(|_: glib::Object, created: i64| {
                    glib::DateTime::from_unix_local(created)
                        .unwrap()
                        .format(
                            // Translators: This is a date time format (https://valadoc.org/glib-2.0/GLib.DateTime.format.html)
                            &gettext("%x %X"),
                        )
                        .unwrap()
                }))
                .bind(&*self.created_row, "value", Some(obj));

            port_bindings_expr
                .chain_closure::<String>(closure!(
                    |_: glib::Object, port_bindings: utils::BoxedStringVec| {
                        port_bindings
                            .iter()
                            .map(|host_port| {
                                format!("<a href='http://{}'>{}</a>", host_port, host_port)
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                ))
                .bind(&*self.port_bindings_label, "label", Some(obj));

            port_bindings_expr
                .chain_closure::<bool>(closure!(
                    |_: glib::Object, port_bindings: utils::BoxedStringVec| !port_bindings
                        .is_empty()
                ))
                .bind(&*self.port_bindings_row, "visible", Some(obj));

            gtk::ClosureExpression::new::<String, _, _>(
                &[
                    &status_expr,
                    &container_expr.chain_property::<model::Container>("up-since"),
                ],
                closure!(
                    |_: glib::Object, status: model::ContainerStatus, up_since: i64| {
                        use model::ContainerStatus::*;

                        match status {
                            Running | Paused => gettext!(
                                // Translators: "{}" is a placeholder for a date time.
                                "Up since {}",
                                glib::DateTime::from_unix_local(up_since)
                                    .unwrap()
                                    .format(
                                        // Translators: This is a date time format (https://valadoc.org/glib-2.0/GLib.DateTime.format.html)
                                        &gettext("%x %X"),
                                    )
                                    .unwrap()
                            ),
                            _ => String::new(),
                        }
                    }
                ),
            )
            .bind(&*self.state_since_label, "label", Some(obj));

            status_expr
                .chain_closure::<String>(closure!(
                    |_: glib::Object, status: model::ContainerStatus| status.to_string()
                ))
                .bind(&*self.status_label, "label", Some(obj));

            let css_classes = self.status_label.css_classes();
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: glib::Object, status: model::ContainerStatus| {
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
                    |_: glib::Object, status: model::ContainerHealthStatus| status.to_string()
                ))
                .bind(&*self.health_status_label, "label", Some(obj));

            let css_classes = self.status_label.css_classes();
            health_status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: glib::Object, status: model::ContainerHealthStatus| {
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

            gtk::ClosureExpression::new::<String, _, _>(
                &[
                    image_expr.chain_property::<model::Image>("repo-tags"),
                    image_expr.chain_property::<model::Image>("id"),
                ],
                closure!(
                    |_: glib::Object, repo_tags: utils::BoxedStringVec, id: &str| {
                        repo_tags
                            .iter()
                            .next()
                            .cloned()
                            .unwrap_or_else(|| id.chars().take(12).collect())
                    }
                ),
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
                .chain_closure::<String>(closure!(
                    |_: glib::Object, image: Option<model::Image>| {
                        match image {
                            None => "waiting",
                            Some(_) => "ready",
                        }
                    }
                ))
                .bind(&*self.image_action_stack, "visible-child-name", Some(obj));

            pod_expr
                .chain_closure::<bool>(closure!(|_: glib::Object, pod: Option<model::Pod>| {
                    pod.is_some()
                }))
                .bind(&*self.pod_row, "visible", Some(obj));

            pod_expr
                .chain_closure::<String>(closure!(|_: glib::Object, pod: Option<model::Pod>| {
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
