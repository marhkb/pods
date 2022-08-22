use gtk::glib;
use gtk::prelude::StaticType;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::model;
use crate::podman;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct ImageData {
        pub(super) architecture: OnceCell<Option<String>>,
        pub(super) author: OnceCell<Option<String>>,
        pub(super) comment: OnceCell<Option<String>>,
        pub(super) config: OnceCell<model::ImageConfig>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageData {
        const NAME: &'static str = "ImageData";
        type Type = super::ImageData;
    }

    impl ObjectImpl for ImageData {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::new(
                        "architecture",
                        "Architecture",
                        "The architecture of the image",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "author",
                        "Author",
                        "The author of the image",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "comment",
                        "Comment",
                        "The author of the image",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecObject::new(
                        "config",
                        "Config",
                        "The config of the image",
                        model::ImageConfig::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                ]
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
                "architecture" => self.architecture.set(value.get().unwrap()).unwrap(),
                "author" => self.author.set(value.get().unwrap()).unwrap(),
                "comment" => self.comment.set(value.get().unwrap()).unwrap(),
                "config" => self.config.set(value.get().unwrap()).unwrap(),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "architecture" => obj.architecture().to_value(),
                "author" => obj.author().to_value(),
                "comment" => obj.comment().to_value(),
                "config" => obj.config().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageData(ObjectSubclass<imp::ImageData>);
}

impl From<podman::models::ImageData> for ImageData {
    fn from(inspect_response: podman::models::ImageData) -> Self {
        glib::Object::new(&[
            ("architecture", &inspect_response.architecture),
            ("author", &inspect_response.author),
            ("comment", &inspect_response.comment),
            (
                "config",
                &model::ImageConfig::from_libpod(inspect_response.config),
            ),
        ])
        .expect("Failed to create ImageData")
    }
}

impl ImageData {
    pub(crate) fn architecture(&self) -> Option<&str> {
        self.imp().architecture.get().unwrap().as_deref()
    }

    pub(crate) fn author(&self) -> Option<&str> {
        self.imp().author.get().unwrap().as_deref()
    }

    pub(crate) fn comment(&self) -> Option<&str> {
        self.imp().comment.get().unwrap().as_deref()
    }

    pub(crate) fn config(&self) -> &model::ImageConfig {
        self.imp().config.get().unwrap()
    }
}
