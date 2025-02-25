use std::cell::OnceCell;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::podman;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ImageSearchResponse)]
    pub(crate) struct ImageSearchResponse {
        #[property(get, set, construct_only, nullable)]
        pub(super) automated: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) description: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) index: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) name: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) official: OnceCell<bool>,
        #[property(get, set, construct_only)]
        pub(super) stars: OnceCell<i64>,
        #[property(get, set, construct_only, nullable)]
        pub(super) tag: OnceCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageSearchResponse {
        const NAME: &'static str = "ImageSearchResponse";
        type Type = super::ImageSearchResponse;
    }

    impl ObjectImpl for ImageSearchResponse {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageSearchResponse(ObjectSubclass<imp::ImageSearchResponse>);
}

impl From<podman::models::RegistrySearchResponse> for ImageSearchResponse {
    fn from(response: podman::models::RegistrySearchResponse) -> Self {
        glib::Object::builder()
            .property("automated", response.automated)
            .property("description", response.description)
            .property("index", response.index)
            .property("name", response.name)
            .property(
                "official",
                response.official.map(|s| !s.is_empty()).unwrap_or(false),
            )
            .property("stars", response.stars.unwrap_or(-1))
            .property("tag", response.tag)
            .build()
    }
}
