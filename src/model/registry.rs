use std::cell::RefCell;

use gtk::glib;
use gtk::prelude::ObjectExt;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct Registry(pub(super) RefCell<String>);

    #[glib::object_subclass]
    impl ObjectSubclass for Registry {
        const NAME: &'static str = "Registry";
        type Type = super::Registry;
    }

    impl ObjectImpl for Registry {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecString::new(
                    "name",
                    "Name",
                    "The name",
                    None,
                    glib::ParamFlags::CONSTRUCT
                        | glib::ParamFlags::READWRITE
                        | glib::ParamFlags::EXPLICIT_NOTIFY,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "name" => self.instance().set_name(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "name" => self.instance().name().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct Registry(ObjectSubclass<imp::Registry>);
}

impl From<&str> for Registry {
    fn from(name: &str) -> Self {
        glib::Object::new::<Self>(&[("name", &name)])
    }
}

impl Registry {
    pub(crate) fn name(&self) -> String {
        self.imp().0.borrow().to_owned()
    }

    pub(crate) fn set_name(&self, value: String) {
        if self.name() == value {
            return;
        }
        self.imp().0.replace(value);
        self.notify("name");
    }
}
