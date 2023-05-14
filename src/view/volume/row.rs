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
    #[template(resource = "/com/github/marhkb/Pods/ui/volume/row.ui")]
    pub(crate) struct Row {
        #[property(get, set = Self::set_volume, construct, nullable)]
        pub(super) volume: RefCell<Option<model::Volume>>,
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[template_child]
        pub(super) writable_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) selinux_drop_down: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) host_path_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) container_path_entry: TemplateChild<gtk::Entry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsVolumeRow";
        type Type = super::Row;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action("volume-row.remove", None, |widget, _, _| {
                if let Some(volume) = widget.volume() {
                    volume.remove_request();
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
        pub(super) fn set_volume(&self, value: Option<model::Volume>) {
            let obj = &*self.obj();

            if obj.volume() == value {
                return;
            }

            let mut bindings = self.bindings.borrow_mut();

            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }

            if let Some(ref volume) = value {
                let binding = volume
                    .bind_property("writable", &*self.writable_switch, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);

                let binding = volume
                    .bind_property("selinux", &*self.selinux_drop_down, "selected")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .transform_to(|_, selinux: model::VolumeSELinux| {
                        Some(
                            match selinux {
                                model::VolumeSELinux::NoLabel => 0_u32,
                                model::VolumeSELinux::Shared => 1_u32,
                                model::VolumeSELinux::Private => 2_u32,
                            }
                            .to_value(),
                        )
                    })
                    .transform_from(|_, position: u32| {
                        Some(
                            match position {
                                0 => model::VolumeSELinux::NoLabel,
                                1 => model::VolumeSELinux::Shared,
                                _ => model::VolumeSELinux::Private,
                            }
                            .to_value(),
                        )
                    })
                    .build();
                bindings.push(binding);

                let binding = volume
                    .bind_property("host-path", &*self.host_path_entry, "text")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);

                let binding = volume
                    .bind_property("container-path", &*self.container_path_entry, "text")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);
            }

            self.volume.replace(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct Row(ObjectSubclass<imp::Row>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Volume> for Row {
    fn from(volume: &model::Volume) -> Self {
        glib::Object::builder().property("volume", volume).build()
    }
}
