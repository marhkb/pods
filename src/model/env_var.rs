use std::cell::RefCell;

use gtk::glib;
use gtk::glib::subclass::Signal;
use gtk::prelude::ObjectExt;
use gtk::prelude::StaticType;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct EnvVar {
        pub(super) key: RefCell<String>,
        pub(super) value: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EnvVar {
        const NAME: &'static str = "EnvVar";
        type Type = super::EnvVar;
    }

    impl ObjectImpl for EnvVar {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("remove-request", &[], <()>::static_type().into()).build()]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::new(
                        "key",
                        "Key",
                        "The key",
                        None,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecString::new(
                        "value",
                        "Value",
                        "The value",
                        None,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
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
                "key" => obj.set_key(value.get().unwrap_or_default()),
                "value" => obj.set_value(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "key" => obj.key().to_value(),
                "value" => obj.value().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct EnvVar(ObjectSubclass<imp::EnvVar>);
}

impl Default for EnvVar {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create EnvVar")
    }
}

impl EnvVar {
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
