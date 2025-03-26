use std::cell::Cell;
use std::cell::RefCell;
use std::panic;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::glib;

use crate::model;
use crate::podman;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ImagePull)]
    pub(crate) struct ImagePull {
        #[property(get, set, construct_only)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set)]
        pub(super) name: RefCell<String>,
        #[property(get, set)]
        pub(super) tag: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePull {
        const NAME: &'static str = "ImagePull";
        type Type = super::ImagePull;
    }

    impl ObjectImpl for ImagePull {
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
    pub(crate) struct ImagePull(ObjectSubclass<imp::ImagePull>);
}

impl ImagePull {
    pub(crate) fn new(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}
