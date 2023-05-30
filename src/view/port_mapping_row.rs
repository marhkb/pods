use std::cell::RefCell;

use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PortMappingRow)]
    #[template(file = "port_mapping_row.ui")]
    pub(crate) struct PortMappingRow {
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[property(get, set = Self::set_port_mapping, construct)]
        pub(super) port_mapping: RefCell<Option<model::PortMapping>>,
        #[template_child]
        pub(super) container_port_adjustment: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub(super) protocol_drop_down: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) ip_address_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) host_port_adjustment: TemplateChild<gtk::Adjustment>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PortMappingRow {
        const NAME: &'static str = "PdsPortMappingRow";
        type Type = super::PortMappingRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action("port-mapping-row.remove", None, |widget, _, _| {
                if let Some(port_mapping) = widget.port_mapping() {
                    port_mapping.remove_request();
                }
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PortMappingRow {
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

    impl WidgetImpl for PortMappingRow {}
    impl ListBoxRowImpl for PortMappingRow {}

    impl PortMappingRow {
        pub(super) fn set_port_mapping(&self, value: Option<model::PortMapping>) {
            let obj = &*self.obj();
            if obj.port_mapping() == value {
                return;
            }

            let mut bindings = self.bindings.borrow_mut();

            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }

            if let Some(ref port_mapping) = value {
                let binding = port_mapping
                    .bind_property("container-port", &*self.container_port_adjustment, "value")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);

                let binding = port_mapping
                    .bind_property("protocol", &*self.protocol_drop_down, "selected")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .transform_to(|_, protocol: model::PortMappingProtocol| {
                        Some(
                            match protocol {
                                model::PortMappingProtocol::Tcp => 0_u32,
                                model::PortMappingProtocol::Udp => 1_u32,
                            }
                            .to_value(),
                        )
                    })
                    .transform_from(|_, position: u32| {
                        Some(
                            if position == 0 {
                                model::PortMappingProtocol::Tcp
                            } else {
                                model::PortMappingProtocol::Udp
                            }
                            .to_value(),
                        )
                    })
                    .build();
                bindings.push(binding);

                let binding = port_mapping
                    .bind_property("ip-address", &*self.ip_address_entry, "text")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);

                let binding = port_mapping
                    .bind_property("host-port", &*self.host_port_adjustment, "value")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);
            }

            self.port_mapping.replace(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct PortMappingRow(ObjectSubclass<imp::PortMappingRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::PortMapping> for PortMappingRow {
    fn from(port_mapping: &model::PortMapping) -> Self {
        glib::Object::builder()
            .property("port-mapping", port_mapping)
            .build()
    }
}
