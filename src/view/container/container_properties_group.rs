use adw::subclass::prelude::PreferencesGroupImpl;
use gettextrs::gettext;
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
    #[template(resource = "/com/github/marhkb/Pods/ui/container-properties-group.ui")]
    pub(crate) struct ContainerPropertiesGroup {
        pub(super) container: WeakRef<model::Container>,
        #[template_child]
        pub(super) id_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) image_name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) image_spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) pod_row: TemplateChild<adw::ActionRow>,
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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerPropertiesGroup {
        const NAME: &'static str = "ContainerPropertiesGroup";
        type Type = super::ContainerPropertiesGroup;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("image.show-details", None, move |widget, _, _| {
                widget.show_image_details();
            });
            klass.install_action("pod.show-details", None, move |widget, _, _| {
                widget.show_pod_details();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerPropertiesGroup {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "container",
                    "Container",
                    "The container of this ContainerPropertiesGroup",
                    model::Container::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "container" => {
                    self.container.set(value.get().unwrap());
                }
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

            self.image_name_label.connect_activate_link(|label, _| {
                label.activate_action("image.show-details", None).unwrap();
                gtk::Inhibit(true)
            });

            let container_expr = Self::Type::this_expression("container");
            let status_expr = container_expr.chain_property::<model::Container>("status");
            let image_expr = container_expr.chain_property::<model::Container>("image");
            let pod_expr = container_expr.chain_property::<model::Container>("pod");
            let port_bindings_expr =
                container_expr.chain_property::<model::Container>("port-bindings");

            container_expr
                .chain_property::<model::Container>("id")
                .chain_closure::<String>(closure!(|_: glib::Object, id: &str| {
                    id.chars().take(12).collect::<String>()
                }))
                .bind(&*self.id_row, "value", Some(obj));

            image_expr
                .chain_property::<model::Image>("repo-tags")
                .chain_closure::<String>(closure!(
                    |_: glib::Object, repo_tags: utils::BoxedStringVec| {
                        repo_tags
                            .iter()
                            .next()
                            .map(|tag| format!("<a href=''>{}</a>", tag))
                            .unwrap_or_default()
                    }
                ))
                .bind(&*self.image_name_label, "label", Some(obj));

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

            image_expr
                .chain_closure::<bool>(closure!(|_: glib::Object, image: Option<model::Image>| {
                    image.is_none()
                }))
                .bind(&*self.image_spinner, "visible", Some(obj));

            pod_expr
                .chain_closure::<bool>(closure!(|_: glib::Object, pod: Option<model::Pod>| {
                    pod.is_some()
                }))
                .bind(&*self.pod_row, "visible", Some(obj));

            pod_expr
                .chain_closure::<String>(closure!(|_: glib::Object, pod: Option<model::Pod>| {
                    pod.as_ref().map(model::Pod::name).unwrap_or_default()
                }))
                .bind(&*self.pod_row, "subtitle", Some(obj));

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
        }
    }

    impl WidgetImpl for ContainerPropertiesGroup {}
    impl PreferencesGroupImpl for ContainerPropertiesGroup {}
}

glib::wrapper! {
    pub(crate) struct ContainerPropertiesGroup(ObjectSubclass<imp::ContainerPropertiesGroup>)
        @extends gtk::Widget, adw::PreferencesGroup;
}

impl ContainerPropertiesGroup {
    pub(crate) fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
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
