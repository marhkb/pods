use std::cell::RefCell;
use std::sync::OnceLock;

use glib::Properties;
// use gtk::glib::subclass::Signal;
use glib::prelude::*;
use glib::subclass::Signal;
use glib::subclass::prelude::*;
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Value)]
    pub(crate) struct Value {
        #[property(name = "value", get, set)]
        pub(super) inner: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Value {
        const NAME: &'static str = "Value";
        type Type = super::Value;
    }

    impl ObjectImpl for Value {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("remove-request").build()])
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
    pub(crate) struct Value(ObjectSubclass<imp::Value>);
}

impl Default for Value {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl Value {
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
