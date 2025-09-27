use std::cell::Cell;
use std::cell::RefCell;
use std::panic;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::glib;

use crate::model;
use crate::podman;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::VolumeCreation)]
    pub(crate) struct VolumeCreation {
        #[property(get, set, construct_only)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set)]
        pub(super) name: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeCreation {
        const NAME: &'static str = "VolumeCreation";
        type Type = super::VolumeCreation;
    }

    impl ObjectImpl for VolumeCreation {
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
    pub(crate) struct VolumeCreation(ObjectSubclass<imp::VolumeCreation>);
}

impl From<&model::Client> for VolumeCreation {
    fn from(value: &model::Client) -> Self {
        glib::Object::builder()
            .property("client", value)
            .property("name", &utils::NAME_GENERATOR.borrow_mut().next().unwrap())
            .build()
    }
}
