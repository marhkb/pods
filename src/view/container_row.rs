use std::cell::RefCell;

use adw::subclass::prelude::{ActionRowImpl, PreferencesRowImpl};
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use once_cell::sync::Lazy;

use crate::{model, utils};

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/container-row.ui")]
    pub(crate) struct ContainerRow {
        pub(super) container: RefCell<Option<model::Container>>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerRow {
        const NAME: &'static str = "ContainerRow";
        type Type = super::ContainerRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "container",
                    "container",
                    "The Container of this ContainerRow",
                    model::Container::static_type(),
                    glib::ParamFlags::READWRITE,
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
                    self.container.replace(value.get().unwrap());
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

            container_expr
                .chain_property::<model::Container>("name")
                .chain_closure::<String>(closure!(|_: glib::Object, name: Option<String>| {
                    utils::escape(&utils::format_option(name))
                }))
                .bind(obj, "title", Some(obj));

            container_expr
                .chain_property::<model::Container>("image-name")
                .chain_closure::<String>(closure!(|_: glib::Object, name: Option<String>| {
                    utils::escape(&utils::format_option(name))
                }))
                .bind(obj, "subtitle", Some(obj));

            container_expr
                .chain_property::<model::Container>("status")
                .chain_closure::<String>(closure!(
                    |_: glib::Object, status: model::ContainerStatus| status.to_string()
                ))
                .bind(&*self.status_label, "label", Some(obj));

            let css_classes = self.status_label.css_classes();
            container_expr
                .chain_property::<model::Container>("status")
                .chain_closure::<Vec<String>>(closure!(
                    |_: glib::Object, status: model::ContainerStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(glib::GString::from(match status {
                                model::ContainerStatus::Configured => "container-status-configured",
                                model::ContainerStatus::Exited => "container-status-exited",
                                model::ContainerStatus::Running => "container-status-running",
                                model::ContainerStatus::Unknown => "container-status-unknown",
                            })))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.status_label, "css-classes", Some(obj));
        }
    }

    impl WidgetImpl for ContainerRow {}
    impl ListBoxRowImpl for ContainerRow {}
    impl PreferencesRowImpl for ContainerRow {}
    impl ActionRowImpl for ContainerRow {}
}

glib::wrapper! {
    pub(crate) struct ContainerRow(ObjectSubclass<imp::ContainerRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

impl From<&model::Container> for ContainerRow {
    fn from(container: &model::Container) -> Self {
        glib::Object::new(&[("container", container)]).expect("Failed to create ContainerRow")
    }
}

impl ContainerRow {
    pub(crate) fn container(&self) -> Option<model::Container> {
        self.imp().container.borrow().clone()
    }
}
