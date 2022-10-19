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
    #[template(resource = "/com/github/marhkb/Pods/ui/volume/row.ui")]
    pub(crate) struct Row {
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
            Self::bind_template(klass);
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
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::Volume>("volume")
                    .flags(
                        glib::ParamFlags::READWRITE
                            | glib::ParamFlags::CONSTRUCT
                            | glib::ParamFlags::EXPLICIT_NOTIFY,
                    )
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "volume" => self.instance().set_volume(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "volume" => self.instance().volume().to_value(),
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

impl From<&model::Volume> for Row {
    fn from(volume: &model::Volume) -> Self {
        glib::Object::new::<Self>(&[("volume", &volume)])
    }
}

impl Row {
    pub(crate) fn volume(&self) -> Option<model::Volume> {
        self.imp().volume.borrow().to_owned()
    }

    pub(crate) fn set_volume(&self, value: Option<model::Volume>) {
        if self.volume() == value {
            return;
        }

        let imp = self.imp();
        let mut bindings = imp.bindings.borrow_mut();

        while let Some(binding) = bindings.pop() {
            binding.unbind();
        }

        if let Some(ref volume) = value {
            let binding = volume
                .bind_property("writable", &*imp.writable_switch, "active")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
            bindings.push(binding);

            let binding = volume
                .bind_property("selinux", &*imp.selinux_drop_down, "selected")
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
                .bind_property("host-path", &*imp.host_path_entry, "text")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
            bindings.push(binding);

            let binding = volume
                .bind_property("container-path", &*imp.container_path_entry, "text")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
            bindings.push(binding);
        }

        imp.volume.replace(value);
        self.notify("volume");
    }
}
