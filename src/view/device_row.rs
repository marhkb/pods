use std::cell::RefCell;

use adw::subclass::prelude::ExpanderRowImpl;
use adw::subclass::prelude::PreferencesRowImpl;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::DeviceRow)]
    #[template(file = "device_row.ui")]
    pub(crate) struct DeviceRow {
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[property(get, set = Self::set_device, nullable)]
        pub(super) device: RefCell<Option<model::Device>>,
        #[template_child]
        pub(super) host_path_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) container_path_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) last_colon_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) options_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) host_path_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) container_path_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) readable_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) writable_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) mknod_switch: TemplateChild<gtk::Switch>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DeviceRow {
        const NAME: &'static str = "PdsDeviceRow";
        type Type = super::DeviceRow;
        type ParentType = adw::ExpanderRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action("device-row.remove", None, |widget, _, _| {
                if let Some(device) = widget.device() {
                    device.remove_request();
                }
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DeviceRow {
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

            let device_expr = Self::Type::this_expression("device");
            let options_expr_arr = &[
                device_expr.chain_property::<model::Device>("readable"),
                device_expr.chain_property::<model::Device>("writable"),
                device_expr.chain_property::<model::Device>("mknod"),
            ];
            let option_active_expr = gtk::ClosureExpression::new::<bool>(
                options_expr_arr,
                closure!(
                    |_: Self::Type, readable: bool, writable: bool, mknod: bool| {
                        readable | writable | mknod
                    }
                ),
            );

            device_expr
                .chain_property::<model::Device>("host-path")
                .chain_closure::<String>(closure!(|_: Self::Type, path: &str| {
                    let path = path.trim();
                    if path.is_empty() { "?" } else { path }.to_string()
                }))
                .bind(&self.host_path_label.get(), "label", Some(obj));

            device_expr
                .chain_property::<model::Device>("container-path")
                .chain_closure::<String>(closure!(|_: Self::Type, path: &str| {
                    let path = path.trim();
                    if path.is_empty() { "?" } else { path }.to_string()
                }))
                .bind(&self.container_path_label.get(), "label", Some(obj));

            option_active_expr.bind(&self.last_colon_label.get(), "visible", Some(obj));
            option_active_expr.bind(&self.options_label.get(), "visible", Some(obj));

            gtk::ClosureExpression::new::<String>(
                options_expr_arr,
                closure!(
                    |_: Self::Type, readable: bool, writable: bool, mknod: bool| {
                        format!(
                            "{}{}{}",
                            if readable { "r" } else { "" },
                            if writable { "w" } else { "" },
                            if mknod { "m" } else { "" },
                        )
                    }
                ),
            )
            .bind(&self.options_label.get(), "label", Some(obj));
        }
    }

    impl WidgetImpl for DeviceRow {}
    impl ListBoxRowImpl for DeviceRow {}
    impl PreferencesRowImpl for DeviceRow {}
    impl ExpanderRowImpl for DeviceRow {}

    impl DeviceRow {
        pub(super) fn set_device(&self, value: Option<model::Device>) {
            let obj = &*self.obj();
            if obj.device() == value {
                return;
            }

            let mut bindings = self.bindings.borrow_mut();

            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }

            if let Some(ref device) = value {
                let binding = device
                    .bind_property("writable", &*self.writable_switch, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);
                let binding = device
                    .bind_property("readable", &*self.readable_switch, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);
                let binding = device
                    .bind_property("mknod", &*self.mknod_switch, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);

                let binding = device
                    .bind_property("host-path", &*self.host_path_entry_row, "text")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);

                let binding = device
                    .bind_property("container-path", &*self.container_path_entry_row, "text")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);
            }

            self.device.replace(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct DeviceRow(ObjectSubclass<imp::DeviceRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ExpanderRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Device> for DeviceRow {
    fn from(device: &model::Device) -> Self {
        glib::Object::builder().property("device", device).build()
    }
}
