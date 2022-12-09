use std::cell::Cell;
use std::cell::RefCell;

use gtk::glib;
use gtk::glib::subclass::Signal;
use gtk::prelude::ObjectExt;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;

mod imp {
    use gtk::prelude::ParamSpecBuilderExt;

    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct Device {
        pub(super) host_path: RefCell<String>,
        pub(super) container_path: RefCell<String>,
        pub(super) writable: Cell<bool>,
        pub(super) readable: Cell<bool>,
        pub(super) mknod: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Device {
        const NAME: &'static str = "Device";
        type Type = super::Device;
    }

    impl ObjectImpl for Device {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("remove-request").build()]);
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::builder("host-path")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecString::builder("container-path")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecBoolean::builder("writable")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecBoolean::builder("readable")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecBoolean::builder("mknod")
                        .explicit_notify()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "host-path" => obj.set_host_path(value.get().unwrap_or_default()),
                "container-path" => obj.set_container_path(value.get().unwrap()),
                "writable" => obj.set_writable(value.get().unwrap()),
                "readable" => obj.set_readable(value.get().unwrap()),
                "mknod" => obj.set_mknod(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "host-path" => obj.host_path().to_value(),
                "container-path" => obj.container_path().to_value(),
                "writable" => obj.writable().to_value(),
                "readable" => obj.readable().to_value(),
                "mknod" => obj.mknod().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct Device(ObjectSubclass<imp::Device>);
}

impl Default for Device {
    fn default() -> Self {
        glib::Object::builder::<Self>()
            .property("readable", &true)
            .property("writable", &false)
            .property("mknod", &false)
            .build()
    }
}

impl Device {
    pub(crate) fn host_path(&self) -> String {
        self.imp().host_path.borrow().to_owned()
    }

    pub(crate) fn set_host_path(&self, value: String) {
        if self.host_path() == value {
            return;
        }
        self.imp().host_path.replace(value);
        self.notify("host-path");
    }

    pub(crate) fn container_path(&self) -> String {
        self.imp().container_path.borrow().to_owned()
    }

    pub(crate) fn set_container_path(&self, value: String) {
        if self.container_path() == value {
            return;
        }
        self.imp().container_path.replace(value);
        self.notify("container-path");
    }

    pub(crate) fn writable(&self) -> bool {
        self.imp().writable.get()
    }

    pub(crate) fn set_writable(&self, value: bool) {
        if self.writable() == value {
            return;
        }
        self.imp().writable.set(value);
        self.notify("writable");
    }

    pub(crate) fn readable(&self) -> bool {
        self.imp().readable.get()
    }

    pub(crate) fn set_readable(&self, value: bool) {
        if self.readable() == value {
            return;
        }
        self.imp().readable.set(value);
        self.notify("readable");
    }

    pub(crate) fn mknod(&self) -> bool {
        self.imp().mknod.get()
    }

    pub(crate) fn set_mknod(&self, value: bool) {
        if self.mknod() == value {
            return;
        }
        self.imp().mknod.set(value);
        self.notify("mknod");
    }

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
