use std::cell::Cell;
use std::cell::RefCell;
use std::sync::OnceLock;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::Signal;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;
use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::PortMapping)]
    pub(crate) struct PortMapping {
        #[property(get, set)]
        pub(super) ip_address: RefCell<String>,
        #[property(get, set)]
        pub(super) host_port: Cell<i32>,
        #[property(get, set, minimum = 1, default = 1)]
        pub(super) container_port: Cell<i32>,
        #[property(get, set, default)]
        pub(super) protocol: Cell<model::PortMappingProtocol>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PortMapping {
        const NAME: &'static str = "PortMapping";
        type Type = super::PortMapping;
    }

    impl ObjectImpl for PortMapping {
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
    pub(crate) struct PortMapping(ObjectSubclass<imp::PortMapping>);
}

impl Default for PortMapping {
    fn default() -> Self {
        glib::Object::builder()
            .property("container-port", 1)
            .build()
    }
}

impl From<engine::dto::PortMapping> for PortMapping {
    fn from(value: engine::dto::PortMapping) -> Self {
        glib::Object::builder()
            .property("ip-address", value.host_ip)
            .property(
                "host-port",
                value.host_port.map(|port| port as i32).unwrap_or(1),
            )
            .property("container-port", value.container_port as i32)
            .property("protocol", model::PortMappingProtocol::from(value.protocol))
            .build()
    }
}

impl From<PortMapping> for engine::dto::PortMapping {
    fn from(value: PortMapping) -> Self {
        Self {
            container_port: value.container_port() as u16,
            host_ip: value.ip_address(),
            host_port: Some(value.host_port() as u16),
            protocol: value.protocol().into(),
        }
    }
}

impl PortMapping {
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
