use std::cell::OnceCell;

use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::Properties;
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Registry)]
    pub(crate) struct Registry {
        #[property(get, set, construct_only)]
        pub(super) name: OnceCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Registry {
        const NAME: &'static str = "Registry";
        type Type = super::Registry;
    }

    impl ObjectImpl for Registry {
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
    pub(crate) struct Registry(ObjectSubclass<imp::Registry>);
}

impl From<&str> for Registry {
    fn from(name: &str) -> Self {
        glib::Object::builder().property("name", name).build()
    }
}
