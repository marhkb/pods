use std::cell::Cell;
use std::cell::RefCell;
use std::fmt;

use gtk::glib;
use gtk::glib::subclass::Signal;
use gtk::prelude::ObjectExt;
use gtk::prelude::ParamSpecBuilderExt;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "VolumeSELinux")]
pub(crate) enum SELinux {
    #[default]
    NoLabel,
    Shared,
    Private,
}

impl fmt::Display for SELinux {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::NoLabel => "",
                Self::Shared => "z",
                Self::Private => "Z",
            }
        )
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct Volume {
        pub(super) host_path: RefCell<String>,
        pub(super) container_path: RefCell<String>,
        pub(super) writable: Cell<bool>,
        pub(super) selinux: Cell<SELinux>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Volume {
        const NAME: &'static str = "Volume";
        type Type = super::Volume;
    }

    impl ObjectImpl for Volume {
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
                    glib::ParamSpecEnum::builder::<SELinux>("selinux").build(),
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
                "selinux" => obj.set_selinux(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "host-path" => obj.host_path().to_value(),
                "container-path" => obj.container_path().to_value(),
                "writable" => obj.writable().to_value(),
                "selinux" => obj.selinux().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct Volume(ObjectSubclass<imp::Volume>);
}

impl Default for Volume {
    fn default() -> Self {
        glib::Object::builder().property("writable", true).build()
    }
}

impl Volume {
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

    pub(crate) fn selinux(&self) -> SELinux {
        self.imp().selinux.get()
    }

    pub(crate) fn set_selinux(&self, value: SELinux) {
        if self.selinux() == value {
            return;
        }
        self.imp().selinux.set(value);
        self.notify("selinux");
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
