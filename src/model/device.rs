use std::cell::Cell;
use std::cell::RefCell;

use glib::Properties;
use gtk::glib;
use gtk::glib::subclass::Signal;
use gtk::prelude::ObjectExt;
use gtk::prelude::ParamSpecBuilderExt;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy as SyncLazy;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Device)]
    pub(crate) struct Device {
        #[property(get, set)]
        pub(super) host_path: RefCell<String>,
        #[property(get, set)]
        pub(super) container_path: RefCell<String>,
        #[property(get, set)]
        pub(super) writable: Cell<bool>,
        #[property(get, set)]
        pub(super) readable: Cell<bool>,
        #[property(get, set)]
        pub(super) mknod: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Device {
        const NAME: &'static str = "Device";
        type Type = super::Device;
    }

    impl ObjectImpl for Device {
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
    pub(crate) struct Device(ObjectSubclass<imp::Device>);
}

impl Default for Device {
    fn default() -> Self {
        glib::Object::builder()
            .property("readable", true)
            .property("writable", false)
            .property("mknod", false)
            .build()
    }
}

impl Device {
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
