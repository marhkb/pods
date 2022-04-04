use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::{StaticType, ToValue};
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::model;
use crate::model::AbstractContainerListExt;
use crate::utils::ToTypedListModel;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct Client {
        pub(super) image_list: OnceCell<model::ImageList>,
        pub(super) container_list: OnceCell<model::ContainerList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Client {
        const NAME: &'static str = "Client";
        type Type = super::Client;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for Client {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "image-list",
                        "Image List",
                        "The list of images",
                        model::ImageList::static_type(),
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecObject::new(
                        "container-list",
                        "Container List",
                        "The list of containers",
                        model::ContainerList::static_type(),
                        glib::ParamFlags::READABLE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image-list" => obj.image_list().to_value(),
                "container-list" => obj.container_list().to_value(),

                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.image_list()
                .connect_image_added(clone!(@weak obj => move |_, image| {
                    obj.container_list()
                        .to_owned()
                        .to_typed_list_model::<model::Container>()
                        .into_iter()
                        .filter(|container| container.image_id() == Some(image.id()))
                        .for_each(|container| {
                            container.set_image(Some(image));
                            image.add_container(container);
                        });
                }));

            obj.container_list()
                .connect_container_added(clone!(@weak obj => move |_, container| {
                    let image = obj.image_list().get_image(container.image_id().unwrap());
                    container.set_image(image.as_ref());
                    if let Some(image) = image {
                        image.add_container(container.to_owned());
                    }
                }));
            obj.container_list().connect_container_removed(
                clone!(@weak obj => move |_, container| {
                    if let Some(image) = container
                        .image_id()
                        .and_then(|id| obj.image_list().get_image(id))
                    {
                        image.remove_container(container.id().unwrap());
                    }
                }),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct Client(ObjectSubclass<imp::Client>);
}

impl Default for Client {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create Client")
    }
}

impl Client {
    pub(crate) fn image_list(&self) -> &model::ImageList {
        self.imp()
            .image_list
            .get_or_init(|| model::ImageList::from(self))
    }

    pub(crate) fn container_list(&self) -> &model::ContainerList {
        self.imp()
            .container_list
            .get_or_init(|| model::ContainerList::from(self))
    }
}
