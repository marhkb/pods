use gtk::glib;
use gtk::prelude::StaticType;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::api;
use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct ImageDetails {
        pub(super) architecture: OnceCell<Option<String>>,
        pub(super) author: OnceCell<Option<String>>,
        pub(super) comment: OnceCell<Option<String>>,
        pub(super) config: OnceCell<model::ImageConfig>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageDetails {
        const NAME: &'static str = "ImageDetails";
        type Type = super::ImageDetails;
    }

    impl ObjectImpl for ImageDetails {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::new(
                        "architecture",
                        "Architecture",
                        "The architecture of this ImageDetails",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "author",
                        "Author",
                        "The author of this ImageDetails",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "comment",
                        "Comment",
                        "The author of this ImageDetails",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecObject::new(
                        "config",
                        "Config",
                        "The config of this ImageDetails",
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
    pub(crate) struct ImageDetails(ObjectSubclass<imp::ImageDetails>);
}

impl From<api::LibpodImageInspectResponse> for ImageDetails {
    fn from(inspect_response: api::LibpodImageInspectResponse) -> Self {
        glib::Object::new(&[
            ("architecture", &inspect_response.architecture),
            ("author", &inspect_response.author),
            ("comment", &inspect_response.comment),
            (
                "config",
                &model::ImageConfig::from_libpod(inspect_response.config.unwrap()),
            ),
        ])
        .expect("Failed to create ImageDetails")
    }
}

impl ImageDetails {
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
