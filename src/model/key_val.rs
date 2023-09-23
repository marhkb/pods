use std::cell::RefCell;

use glib::once_cell::sync::Lazy as SyncLazy;
use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::subclass::Signal;
use glib::Properties;
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::KeyVal)]

    pub(crate) struct KeyVal {
        #[property(get, set)]
        pub(super) key: RefCell<String>,
        #[property(get, set)]
        pub(super) value: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for KeyVal {
        const NAME: &'static str = "KeyVal";
        type Type = super::KeyVal;
    }

    impl ObjectImpl for KeyVal {
        fn signals() -> &'static [Signal] {
            static SIGNALS: SyncLazy<Vec<Signal>> =
                SyncLazy::new(|| vec![Signal::builder("remove-request").build()]);
            SIGNALS.as_ref()
        }

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
    pub(crate) struct KeyVal(ObjectSubclass<imp::KeyVal>);
}

impl Default for KeyVal {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl KeyVal {
    pub(crate) fn remove_request(&self) {
        self.emit_by_name::<()>("remove-request", &[]);
    }

    pub(crate) fn connect_remove_request<F: Fn(&Self) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("remove-request", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);

            None
        })
    }
}
