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
    #[properties(wrapper_type = super::PortBindingCreation)]
    pub(crate) struct PortBindingCreation {
        #[property(get, set)]
        pub(super) ip_address: RefCell<String>,
        #[property(get, set)]
        pub(super) host_port: Cell<i32>,
        #[property(get, set, default)]
        pub(super) protocol: Cell<model::PortMappingProtocol>,
        #[property(get, set, minimum = 1, default = 1)]
        pub(super) target_port: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PortBindingCreation {
        const NAME: &'static str = "PortBindingCreation";
        type Type = super::PortBindingCreation;
    }

    impl ObjectImpl for PortBindingCreation {
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
    pub(crate) struct PortBindingCreation(ObjectSubclass<imp::PortBindingCreation>);
}

impl Default for PortBindingCreation {
    fn default() -> Self {
        glib::Object::builder().property("target-port", 1).build()
    }
}

impl From<PortBindingCreation> for engine::dto::PortBinding {
    fn from(value: PortBindingCreation) -> Self {
        Self {
            target_port: value.target_port() as u16,
            host_ip: value.ip_address(),
            host_port: Some(value.host_port() as u16),
            protocol: value.protocol().into(),
        }
    }
}

impl From<&engine::dto::PortBinding> for PortBindingCreation {
    fn from(value: &engine::dto::PortBinding) -> Self {
        glib::Object::builder()
            .property("ip-address", &value.host_ip)
            .property(
                "host-port",
                value.host_port.map(|port| port as i32).unwrap_or(1),
            )
            .property("protocol", model::PortMappingProtocol::from(value.protocol))
            .property("target-port", value.target_port as i32)
            .build()
    }
}
impl PortBindingCreation {
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
