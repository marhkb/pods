use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy as SyncLazy;

mod imp {
    use super::*;

    #[derive(Copy, Clone, Debug)]
    pub(crate) struct Selectable(glib::gobject_ffi::GTypeInterface);

    #[glib::object_interface]
    unsafe impl ObjectInterface for Selectable {
        const NAME: &'static str = "Selectable";

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: SyncLazy<Vec<glib::ParamSpec>> = SyncLazy::new(|| {
                vec![glib::ParamSpecBoolean::builder("selected")
                    .explicit_notify()
                    .build()]
            });
            PROPERTIES.as_ref()
        }
    }
}

glib::wrapper! { pub(crate) struct Selectable(ObjectInterface<imp::Selectable>); }

pub(crate) trait SelectableExt: glib::IsA<Selectable> {
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
