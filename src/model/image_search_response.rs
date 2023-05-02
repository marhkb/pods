use glib::ObjectExt;
use glib::Properties;
use gtk::glib;
use gtk::subclass::prelude::*;
use once_cell::unsync::OnceCell as UnsyncOnceCell;

use crate::podman;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ImageSearchResponse)]
    pub(crate) struct ImageSearchResponse {
        #[property(get, set, construct_only, nullable)]
        pub(super) automated: UnsyncOnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) description: UnsyncOnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) index: UnsyncOnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) name: UnsyncOnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) official: UnsyncOnceCell<Option<String>>,
        #[property(get, set, construct_only)]
        pub(super) stars: UnsyncOnceCell<i64>,
        #[property(get, set, construct_only, nullable)]
        pub(super) tag: UnsyncOnceCell<Option<String>>,
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
            .property("official", response.official)
            .property("stars", response.stars.unwrap_or(-1))
            .property("tag", response.tag)
            .build()
    }
}
