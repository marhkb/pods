use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::panic;

use adw::prelude::*;
use adw::subclass::prelude::*;
use futures::future;
use glib::Properties;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::podman;
use crate::rt;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ImageSearch)]
    pub(crate) struct ImageSearch {
        #[property(get, set, construct_only)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set)]
        pub(super) name: RefCell<String>,
        #[property(get = Self::results)]
        pub(super) results: OnceCell<gio::ListStore>,
        #[property(get, set)]
        pub(super) selected: Cell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageSearch {
        const NAME: &'static str = "ImageSearch";
        type Type = super::ImageSearch;
    }

    impl ObjectImpl for ImageSearch {
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

    impl ImageSearch {
        fn results(&self) -> gio::ListStore {
            self.results
                .get_or_init(gio::ListStore::new::<model::ImageSearchResponse>)
                .to_owned()
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageSearch(ObjectSubclass<imp::ImageSearch>);
}

impl From<&model::Client> for ImageSearch {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}
