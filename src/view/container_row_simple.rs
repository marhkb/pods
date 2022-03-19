use gtk::glib::{closure, WeakRef};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use once_cell::sync::Lazy;

use crate::{model, utils};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/container-row-simple.ui")]
    pub(crate) struct ContainerRowSimple {
        pub(super) container: WeakRef<model::Container>,
        #[template_child]
        pub(super) name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerRowSimple {
        const NAME: &'static str = "ContainerRowSimple";
        type Type = super::ContainerRowSimple;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerRowSimple {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "container",
                    "container",
                    "The Container of this ContainerRowSimple",
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

            container_expr
                .chain_property::<model::Container>("name")
                .chain_closure::<String>(closure!(|_: glib::Object, name: Option<String>| {
                    utils::escape(&utils::format_option(name))
                }))
                .bind(&*self.name_label, "label", Some(obj));

            status_expr
                .chain_closure::<String>(closure!(
                    |_: glib::Object, status: model::ContainerStatus| status.to_string()
                ))
                .bind(&*self.status_label, "label", Some(obj));

            let css_classes = self.status_label.css_classes();
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: glib::Object, status: model::ContainerStatus| {
                        use model::ContainerStatus::*;

                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(glib::GString::from(match status {
                                Configured => "container-status-configured",
                                Created => "container-status-created",
                                Dead => "container-status-dead",
                                Exited => "container-status-exited",
                                Paused => "container-status-paused",
                                Removing => "container-status-removing",
                                Restarting => "container-status-restarting",
                                Running => "container-status-running",
                                Stopped => "container-status-stopped",
                                Stopping => "container-status-stopping",
                                Unknown => "container-status-unknown",
                            })))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.status_label, "css-classes", Some(obj));
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.name_label.unparent();
            self.status_label.unparent();
        }
    }

    impl WidgetImpl for ContainerRowSimple {}
}

glib::wrapper! {
    pub(crate) struct ContainerRowSimple(ObjectSubclass<imp::ContainerRowSimple>)
        @extends gtk::Widget;
}

impl From<Option<&model::Container>> for ContainerRowSimple {
    fn from(container: Option<&model::Container>) -> Self {
        glib::Object::new(&[("container", &container)])
            .expect("Failed to create ContainerRowSimple")
    }
}

impl ContainerRowSimple {
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
}
