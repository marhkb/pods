use std::cell::RefCell;

use gtk::glib;
use gtk::glib::subclass::Signal;
use gtk::prelude::ObjectExt;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;

mod imp {
    use gtk::prelude::ParamSpecBuilderExt;

    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct KeyVal {
        pub(super) key: RefCell<String>,
        pub(super) value: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for KeyVal {
        const NAME: &'static str = "KeyVal";
        type Type = super::KeyVal;
    }

    impl ObjectImpl for KeyVal {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("remove-request").build()]);
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::builder("key")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecString::builder("value")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "key" => obj.set_key(value.get().unwrap_or_default()),
                "value" => obj.set_value(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "key" => obj.key().to_value(),
                "value" => obj.value().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct KeyVal(ObjectSubclass<imp::KeyVal>);
}

impl Default for KeyVal {
    fn default() -> Self {
        glib::Object::builder::<Self>().build()
    }
}

impl KeyVal {
    pub(crate) fn key(&self) -> String {
        self.imp().key.borrow().to_owned()
    }

    pub(crate) fn set_key(&self, value: String) {
        if self.key() == value {
            return;
        }
        self.imp().key.replace(value);
        self.notify("key");
    }

    pub(crate) fn value(&self) -> String {
        self.imp().value.borrow().to_owned()
    }

    pub(crate) fn set_value(&self, value: String) {
        if self.value() == value {
            return;
        }
        self.imp().value.replace(value);
        self.notify("value");
    }

    pub(crate) fn remove_request(&self) {
        self.emit_by_name::<()>("remove-request", &[]);
    }

    pub(crate) fn connect_remove_request<F: Fn(&Self) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("remove-request", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);

            None
        })
    }
}
