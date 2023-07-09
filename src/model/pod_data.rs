use std::cell::OnceCell;

use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::Properties;
use gtk::glib;

use crate::podman;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::PodData)]
    pub(crate) struct PodData {
        #[property(get, set, construct_only)]
        pub(super) hostname: OnceCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodData {
        const NAME: &'static str = "PodData";
        type Type = super::PodData;
    }

    impl ObjectImpl for PodData {
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
    pub(crate) struct PodData(ObjectSubclass<imp::PodData>);
}

impl From<podman::models::InspectPodData> for PodData {
    fn from(data: podman::models::InspectPodData) -> Self {
        glib::Object::builder()
            .property("hostname", data.hostname.unwrap_or_default())
            .build()
    }
}
