use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::ExpanderRowImpl;
use adw::subclass::prelude::PreferencesRowImpl;
use adw::subclass::prelude::*;
use glib::Properties;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PortBindingCreationRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/port_binding_creation_row.ui")]
    pub(crate) struct PortBindingCreationRow {
        pub(super) bindings: RefCell<Vec<glib::Binding>>,

        #[property(get, set = Self::set_port_binding, construct)]
        pub(super) port_binding: RefCell<Option<model::PortBindingCreation>>,
        #[template_child]
        pub(super) protocol_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) ip_address_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) host_port_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) target_port_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) protocol_combo_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) ip_address_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) host_port_adjustment: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub(super) target_port_adjustment: TemplateChild<gtk::Adjustment>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PortBindingCreationRow {
        const NAME: &'static str = "PdsPortBindingCreationRow";
        type Type = super::PortBindingCreationRow;
        type ParentType = adw::ExpanderRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action("port-binding-creation-row.remove", None, |widget, _, _| {
                if let Some(port_binding) = widget.port_binding() {
                    port_binding.remove_request();
                }
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PortBindingCreationRow {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let port_binding_expr = Self::Type::this_expression("port-binding");

            port_binding_expr
                .chain_property::<model::PortBindingCreation>("protocol")
                .chain_closure::<String>(closure!(
                    |_: Self::Type, protocol: model::PortMappingProtocol| protocol.to_string()
                ))
                .bind(&self.protocol_label.get(), "label", Some(obj));

            port_binding_expr
                .chain_property::<model::PortBindingCreation>("ip-address")
                .chain_closure::<String>(closure!(|_: Self::Type, ip_address: &str| {
                    let ip_address = ip_address.trim();
                    if ip_address.is_empty() {
                        "0.0.0.0"
                    } else {
                        ip_address
                    }
                    .to_string()
                }))
                .bind(&self.ip_address_label.get(), "label", Some(obj));

            port_binding_expr
                .chain_property::<model::PortBindingCreation>("host-port")
                .bind(&self.host_port_label.get(), "label", Some(obj));

            port_binding_expr
                .chain_property::<model::PortBindingCreation>("target-port")
                .bind(&self.target_port_label.get(), "label", Some(obj));
        }
    }

    impl WidgetImpl for PortBindingCreationRow {}
    impl ListBoxRowImpl for PortBindingCreationRow {}
    impl PreferencesRowImpl for PortBindingCreationRow {}
    impl ExpanderRowImpl for PortBindingCreationRow {}

    #[gtk::template_callbacks]
    impl PortBindingCreationRow {
        pub(super) fn set_port_binding(&self, value: Option<model::PortBindingCreation>) {
            let obj = &*self.obj();
            if obj.port_binding() == value {
                return;
            }

            let mut bindings = self.bindings.borrow_mut();

            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }

            if let Some(ref port_binding) = value {
                let binding = port_binding
                    .bind_property("protocol", &*self.protocol_combo_row, "selected")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .transform_to(|_, protocol: model::PortMappingProtocol| {
                        Some(
                            match protocol {
                                model::PortMappingProtocol::Tcp => 0_u32,
                                model::PortMappingProtocol::Udp => 1_u32,
                                model::PortMappingProtocol::Sctp => 2_u32,
                            }
                            .to_value(),
                        )
                    })
                    .transform_from(|_, position: u32| {
                        match position {
                            0 => model::PortMappingProtocol::Tcp,
                            1 => model::PortMappingProtocol::Udp,
                            _ => model::PortMappingProtocol::Sctp,
                        }
                        .to_value()
                        .into()
                    })
                    .build();
                bindings.push(binding);

                let binding = port_binding
                    .bind_property("ip-address", &*self.ip_address_entry_row, "text")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);

                let binding = port_binding
                    .bind_property("host-port", &*self.host_port_adjustment, "value")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);

                let binding = port_binding
                    .bind_property("target-port", &*self.target_port_adjustment, "value")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);
            }

            self.port_binding.replace(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct PortBindingCreationRow(ObjectSubclass<imp::PortBindingCreationRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ExpanderRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::PortBindingCreation> for PortBindingCreationRow {
    fn from(port_binding: &model::PortBindingCreation) -> Self {
        glib::Object::builder()
            .property("port-binding", port_binding)
            .build()
    }
}
