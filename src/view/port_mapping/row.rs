use std::cell::RefCell;

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/port-mapping/row.ui")]
    pub(crate) struct Row {
        pub(super) port_mapping: RefCell<Option<model::PortMapping>>,
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[template_child]
        pub(super) container_port_adjustment: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub(super) protocol_combo_box: TemplateChild<gtk::ComboBoxText>,
        #[template_child]
        pub(super) ip_address_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) host_port_adjustment: TemplateChild<gtk::Adjustment>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsPortMappingRow";
        type Type = super::Row;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
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

    impl ObjectImpl for Row {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "port-mapping",
                    "Port Mapping",
                    "The underlying port mappings",
                    model::PortMapping::static_type(),
                    glib::ParamFlags::READWRITE
                        | glib::ParamFlags::CONSTRUCT
                        | glib::ParamFlags::EXPLICIT_NOTIFY,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "port-mapping" => obj.set_port_mapping(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "port-mapping" => obj.port_mapping().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for Row {}
    impl ListBoxRowImpl for Row {}
}

glib::wrapper! {
    pub(crate) struct Row(ObjectSubclass<imp::Row>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::PortMapping> for Row {
    fn from(port_mapping: &model::PortMapping) -> Self {
        glib::Object::new(&[("port-mapping", &port_mapping)])
            .expect("Failed to create PdsPortMappingRow")
    }
}

impl Row {
    pub(crate) fn port_mapping(&self) -> Option<model::PortMapping> {
        self.imp().port_mapping.borrow().to_owned()
    }

    pub(crate) fn set_port_mapping(&self, value: Option<model::PortMapping>) {
        if self.port_mapping() == value {
            return;
        }

        let imp = self.imp();
        let mut bindings = imp.bindings.borrow_mut();

        while let Some(binding) = bindings.pop() {
            binding.unbind();
        }

        if let Some(ref port_mapping) = value {
            let binding = port_mapping
                .bind_property("container-port", &*imp.container_port_adjustment, "value")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
            bindings.push(binding);

            let binding = port_mapping
                .bind_property("protocol", &*imp.protocol_combo_box, "active")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .transform_to(|_, value| {
                    Some(
                        match value.get::<model::PortMappingProtocol>().unwrap() {
                            model::PortMappingProtocol::Tcp => 0,
                            model::PortMappingProtocol::Udp => 1,
                        }
                        .to_value(),
                    )
                })
                .transform_from(|_, value| {
                    Some(
                        if value.get::<i32>().unwrap() == 0 {
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
                .bind_property("ip-address", &*imp.ip_address_entry, "text")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
            bindings.push(binding);

            let binding = port_mapping
                .bind_property("host-port", &*imp.host_port_adjustment, "value")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
            bindings.push(binding);
        }

        imp.port_mapping.replace(value);
        self.notify("port-mapping");
    }
}
