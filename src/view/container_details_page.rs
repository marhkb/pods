use gtk::glib::{closure, WeakRef};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use once_cell::sync::Lazy;

use crate::{model, utils, view};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/container-details-page.ui")]
    pub(crate) struct ContainerDetailsPage {
        pub(super) container: WeakRef<model::Container>,
        #[template_child]
        pub(super) leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub(super) id_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) image_name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) image_spinner: TemplateChild<gtk::Spinner>,
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
            klass.install_action("image.show-details", None, move |widget, _, _| {
                widget.show_details();
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

            image_expr
                .chain_closure::<bool>(closure!(|_: glib::Object, image: Option<model::Image>| {
                    image.is_none()
                }))
                .bind(&*self.image_spinner, "visible", Some(obj));

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
                            .chain(Some(glib::GString::from(view::container_status_css_class(
                                status,
                            ))))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.status_label, "css-classes", Some(obj));
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.leaflet.unparent();
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
        utils::find_leaflet_overview(self).hide_details();
    }

    fn show_details(&self) {
        utils::find_leaflet_overview(&*self.imp().leaflet).show_details(
            &view::ImageDetailsPage::from(&self.container().unwrap().image().unwrap()),
        );
    }
}
