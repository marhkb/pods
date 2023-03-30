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
    #[properties(wrapper_type = super::Row)]
    #[template(resource = "/com/github/marhkb/Pods/ui/device/row.ui")]
    pub(crate) struct Row {
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[property(get, set = Self::set_device, explicit_notify, nullable)]
        pub(super) device: RefCell<Option<model::Device>>,
        #[template_child]
        pub(super) writable_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) readable_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) mknod_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) host_path_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) container_path_entry: TemplateChild<gtk::Entry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsDeviceRow";
        type Type = super::Row;
        type ParentType = gtk::ListBoxRow;

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

    impl ObjectImpl for Row {
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

    impl WidgetImpl for Row {}
    impl ListBoxRowImpl for Row {}

    impl Row {
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
                    .bind_property("host-path", &*self.host_path_entry, "text")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);

                let binding = device
                    .bind_property("container-path", &*self.container_path_entry, "text")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);
            }

            self.device.replace(value);
            obj.notify("device");
        }
    }
}

glib::wrapper! {
    pub(crate) struct Row(ObjectSubclass<imp::Row>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Device> for Row {
    fn from(device: &model::Device) -> Self {
        glib::Object::builder().property("device", device).build()
    }
}
