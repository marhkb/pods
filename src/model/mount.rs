use std::cell::Cell;
use std::cell::RefCell;
use std::fmt;
use std::sync::OnceLock;

use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::subclass::Signal;
use glib::Properties;
use gtk::glib;

use crate::model;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "MountType")]
pub(crate) enum MountType {
    #[default]
    Bind,
    Volume,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "MountSELinux")]
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
    #[properties(wrapper_type = super::Mount)]
    pub(crate) struct Mount {
        #[property(get, set, construct_only)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, builder(MountType::default()))]
        pub(super) mount_type: Cell<MountType>,
        // Used if MountType::Volume is set
        #[property(get, set, nullable)]
        pub(super) volume: glib::WeakRef<model::Volume>,
        // Used if MountType::Bind is set
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
    impl ObjectSubclass for Mount {
        const NAME: &'static str = "Mount";
        type Type = super::Mount;
    }

    impl ObjectImpl for Mount {
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
    pub(crate) struct Mount(ObjectSubclass<imp::Mount>);
}

impl From<&model::Client> for Mount {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("writable", true)
            .build()
    }
}

impl Mount {
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
