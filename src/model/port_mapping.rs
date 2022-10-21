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
#[enum_type(name = "PortMappingProtocol")]
pub(crate) enum Protocol {
    #[default]
    Tcp,
    Udp,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Tcp => "TCP",
                Self::Udp => "UDP",
            }
        )
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct PortMapping {
        pub(super) ip_address: RefCell<String>,
        pub(super) host_port: Cell<i32>,
        pub(super) container_port: Cell<i32>,
        pub(super) protocol: Cell<Protocol>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PortMapping {
        const NAME: &'static str = "PortMapping";
        type Type = super::PortMapping;
    }

    impl ObjectImpl for PortMapping {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("remove-request").build()]);
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::builder("ip-address")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecInt::builder("host-port")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecInt::builder("container-port")
                        .minimum(1)
                        .default_value(1)
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecEnum::builder::<Protocol>("protocol", Protocol::default())
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.instance();
            match pspec.name() {
                "ip-address" => obj.set_ip_address(value.get().unwrap_or_default()),
                "host-port" => obj.set_host_port(value.get().unwrap()),
                "container-port" => obj.set_container_port(value.get().unwrap()),
                "protocol" => obj.set_protocol(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.instance();
            match pspec.name() {
                "ip-address" => obj.ip_address().to_value(),
                "host-port" => obj.host_port().to_value(),
                "container-port" => obj.container_port().to_value(),
                "protocol" => obj.protocol().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct PortMapping(ObjectSubclass<imp::PortMapping>);
}

impl Default for PortMapping {
    fn default() -> Self {
        glib::Object::builder::<Self>().build()
    }
}

impl PortMapping {
    pub(crate) fn ip_address(&self) -> String {
        self.imp().ip_address.borrow().to_owned()
    }

    pub(crate) fn set_ip_address(&self, value: String) {
        if self.ip_address() == value {
            return;
        }
        self.imp().ip_address.replace(value);
        self.notify("ip-address");
    }

    pub(crate) fn host_port(&self) -> i32 {
        self.imp().host_port.get()
    }

    pub(crate) fn set_host_port(&self, value: i32) {
        if self.host_port() == value {
            return;
        }
        self.imp().host_port.set(value);
        self.notify("host-port");
    }

    pub(crate) fn container_port(&self) -> i32 {
        self.imp().container_port.get()
    }

    pub(crate) fn set_container_port(&self, value: i32) {
        if self.container_port() == value {
            return;
        }
        self.imp().container_port.set(value);
        self.notify("container-port");
    }

    pub(crate) fn protocol(&self) -> Protocol {
        self.imp().protocol.get()
    }

    pub(crate) fn set_protocol(&self, value: Protocol) {
        if self.protocol() == value {
            return;
        }
        self.imp().protocol.set(value);
        self.notify("protocol");
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
