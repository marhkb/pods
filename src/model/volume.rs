use std::cell::Cell;
use std::cell::RefCell;
use std::fmt;

use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::subclass::Signal;
use glib::Properties;
use gtk::glib;
use once_cell::sync::Lazy as SyncLazy;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "VolumeSELinux")]
pub(crate) enum SELinux {
    #[default]
    NoLabel,
    Shared,
    Private,
}

impl AsRef<str> for SELinux {
    fn as_ref(&self) -> &str {
        match self {
            Self::NoLabel => "",
            Self::Shared => "z",
            Self::Private => "Z",
        }
    }
}

impl fmt::Display for SELinux {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Volume)]
    pub(crate) struct Volume {
        #[property(get, set)]
        pub(super) host_path: RefCell<String>,
        #[property(get, set)]
        pub(super) container_path: RefCell<String>,
        #[property(get, set, construct)]
        pub(super) writable: Cell<bool>,
        #[property(get, set, builder(SELinux::default()))]
        pub(super) selinux: Cell<SELinux>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Volume {
        const NAME: &'static str = "Volume";
        type Type = super::Volume;
    }

    impl ObjectImpl for Volume {
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
    pub(crate) struct Volume(ObjectSubclass<imp::Volume>);
}

impl Default for Volume {
    fn default() -> Self {
        glib::Object::builder().property("writable", true).build()
    }
}

impl Volume {
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
