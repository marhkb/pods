use std::cell::OnceCell;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::PodDetails)]
    pub(crate) struct PodDetails {
        #[property(get, set, construct_only)]
        pub(super) hostname: OnceCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodDetails {
        const NAME: &'static str = "PodDetails";
        type Type = super::PodDetails;
    }

    impl ObjectImpl for PodDetails {
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
    pub(crate) struct PodDetails(ObjectSubclass<imp::PodDetails>);
}

impl From<engine::dto::PodDetails> for PodDetails {
    fn from(dto: engine::dto::PodDetails) -> Self {
        glib::Object::builder()
            .property("hostname", dto.hostname)
            .build()
    }
}

impl PodDetails {
    pub(crate) fn update(&self, _dto: engine::dto::PodDetails) {}
}
