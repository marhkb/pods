use std::cell::Cell;
use std::cell::RefCell;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;
use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::PortBinding)]
    pub(crate) struct PortBinding {
        #[property(get, set, construct_only, nullable)]
        pub(super) port_binding_list: glib::WeakRef<model::PortBindingList>,

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
    impl ObjectSubclass for PortBinding {
        const NAME: &'static str = "PortBinding";
        type Type = super::PortBinding;
    }

    impl ObjectImpl for PortBinding {
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
    pub(crate) struct PortBinding(ObjectSubclass<imp::PortBinding>);
}

impl Default for PortBinding {
    fn default() -> Self {
        glib::Object::builder().property("target-port", 1).build()
    }
}

impl From<PortBinding> for engine::dto::PortBinding {
    fn from(value: PortBinding) -> Self {
        Self {
            host_ip: value.ip_address(),
            host_port: Some(value.host_port() as u16),
            protocol: value.protocol().into(),
            target_port: value.target_port() as u16,
        }
    }
}

impl PortBinding {
    pub(crate) fn new(
        port_binding_list: &model::PortBindingList,
        value: engine::dto::PortBinding,
    ) -> Self {
        glib::Object::builder()
            .property("port-binding-list", port_binding_list)
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
