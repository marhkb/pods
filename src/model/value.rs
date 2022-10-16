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
    pub(crate) struct Value(pub(super) RefCell<String>);

    #[glib::object_subclass]
    impl ObjectSubclass for Value {
        const NAME: &'static str = "Value";
        type Type = super::Value;
    }

    impl ObjectImpl for Value {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("remove-request", &[], <()>::static_type().into()).build()]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecString::new(
                    "value",
                    "Value",
                    "The value",
                    None,
                    glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                )]
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
                "value" => obj.set_value(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "value" => obj.value().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct Value(ObjectSubclass<imp::Value>);
}

impl Default for Value {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create Value")
    }
}

impl Value {
    pub(crate) fn value(&self) -> String {
        self.imp().0.borrow().to_owned()
    }

    pub(crate) fn set_value(&self, value: String) {
        if self.value() == value {
            return;
        }
        self.imp().0.replace(value);
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
