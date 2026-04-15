use std::sync::OnceLock;

use gio::prelude::*;
use glib::subclass::prelude::*;
use gtk::gio;
use gtk::glib;

mod imp {
    use super::*;

    #[allow(dead_code)]
    #[derive(Copy, Clone, Debug)]
    pub(crate) struct SuggestionItemClass(glib::gobject_ffi::GTypeInterface);

    unsafe impl InterfaceStruct for SuggestionItemClass {
        type Type = SuggestionItem;
    }

    pub(crate) struct SuggestionItem;

    #[glib::object_interface]
    impl ObjectInterface for SuggestionItem {
        const NAME: &'static str = "SuggestionItem";
        type Interface = SuggestionItemClass;

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecString::builder("name").read_only().build(),
                    glib::ParamSpecString::builder("suggestion-postfix")
                        .read_only()
                        .build(),
                ]
            })
        }
    }
}

glib::wrapper! {
    pub(crate) struct SuggestionItem(ObjectInterface<imp::SuggestionItem>);
}

impl SuggestionItem {
    pub(crate) fn name(&self) -> String {
        self.property("name")
    }

    pub(crate) fn suggestion_postfix(&self) -> Option<String> {
        self.property("suggestion-postfix")
    }
}

unsafe impl<T: ObjectSubclass> IsImplementable<T> for SuggestionItem {}
