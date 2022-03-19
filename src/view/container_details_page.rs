use gtk::glib::{closure, WeakRef};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use once_cell::sync::Lazy;

use crate::window::Window;
use crate::{model, view};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/container-details-page.ui")]
    pub(crate) struct ContainerDetailsPage {
        pub(super) container: WeakRef<model::Container>,
        #[template_child]
        pub(super) header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub(super) preferences_page: TemplateChild<adw::PreferencesPage>,
        #[template_child]
        pub(super) id_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) image_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) name_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerDetailsPage {
        const NAME: &'static str = "ContainerDetailsPage";
        type Type = super::ContainerDetailsPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("navigation.back", None, move |widget, _, _| {
                widget.navigate_back();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerDetailsPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "container",
                    "Container",
                    "The container of this ContainerDetailsPage",
                    model::Image::static_type(),
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

            let container_expr = Self::Type::this_expression("container");
            let status_expr = container_expr.chain_property::<model::Container>("status");

            container_expr
                .chain_property::<model::Container>("id")
                .chain_closure::<String>(closure!(|_: glib::Object, id: &str| {
                    id.chars().take(12).collect::<String>()
                }))
                .bind(&*self.id_row, "value", Some(obj));

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
            self.header_bar.unparent();
            self.preferences_page.unparent();
        }
    }

    impl WidgetImpl for ContainerDetailsPage {}
}

glib::wrapper! {
    pub(crate) struct ContainerDetailsPage(ObjectSubclass<imp::ContainerDetailsPage>) @extends gtk::Widget;
}

impl From<&model::Container> for ContainerDetailsPage {
    fn from(image: &model::Container) -> Self {
        glib::Object::new(&[("container", image)]).expect("Failed to create ContainerDetailsPage")
    }
}

impl ContainerDetailsPage {
    pub(crate) fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    fn navigate_back(&self) {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .hide_details();
    }
}
