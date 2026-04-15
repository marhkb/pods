use std::cell::OnceCell;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;
use crate::model;

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
        pub(super) stars: OnceCell<u64>,
        #[property(get, set, construct_only, nullable)]
        pub(super) suggestion_postfix: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) tag: OnceCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageSearchResponse {
        const NAME: &'static str = "ImageSearchResponse";
        type Type = super::ImageSearchResponse;
        type Interfaces = (model::SuggestionItem,);
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
    pub(crate) struct ImageSearchResponse(ObjectSubclass<imp::ImageSearchResponse>)
        @implements model::SuggestionItem;
}

impl From<engine::dto::ImageSearchResponseItem> for ImageSearchResponse {
    fn from(value: engine::dto::ImageSearchResponseItem) -> Self {
        glib::Object::builder()
            .property("automated", value.automated)
            .property("description", value.description)
            .property("index", value.index)
            .property("name", value.name)
            .property("official", value.is_official)
            .property("stars", value.stars)
            .property("suggestion-postfix", ":latest")
            .property("tag", value.tag)
            .build()
    }
}
