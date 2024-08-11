use std::sync::OnceLock;

use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

mod imp {
    use super::*;

    #[allow(dead_code)]
    #[derive(Copy, Clone, Debug)]
    pub(crate) struct SelectableClass(glib::gobject_ffi::GTypeInterface);

    unsafe impl InterfaceStruct for SelectableClass {
        type Type = Selectable;
    }

    pub(crate) struct Selectable;

    #[glib::object_interface]
    impl ObjectInterface for Selectable {
        const NAME: &'static str = "Selectable";
        type Interface = SelectableClass;

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![glib::ParamSpecBoolean::builder("selected")
                    .explicit_notify()
                    .build()]
            })
        }
    }
}

glib::wrapper! { pub(crate) struct Selectable(ObjectInterface<imp::Selectable>); }

pub(crate) trait SelectableExt: IsA<Selectable> {
    fn is_selected(&self) -> bool;

    fn set_selected(&self, value: bool);

    fn select(&self) {
        self.set_selected(!self.is_selected());
    }
}

impl<T: IsA<Selectable>> SelectableExt for T {
    fn is_selected(&self) -> bool {
        self.property("selected")
    }

    fn set_selected(&self, value: bool) {
        self.set_property("selected", value);
    }
}

unsafe impl<T: ObjectSubclass> IsImplementable<T> for Selectable {}
