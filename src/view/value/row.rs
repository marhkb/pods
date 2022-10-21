use std::cell::RefCell;

use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::glib;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;

const ACTION_REMOVE: &str = "value-row.remove";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/value/row.ui")]
    pub(crate) struct Row {
        pub(super) value: RefCell<Option<model::Value>>,
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsValueRow";
        type Type = super::Row;
        type ParentType = adw::EntryRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action(ACTION_REMOVE, None, |widget, _, _| {
                if let Some(value) = widget.value() {
                    value.remove_request();
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
                vec![glib::ParamSpecObject::builder::<model::Value>("value")
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
                "value" => self.instance().set_value(value.get().unwrap_or_default()),
                other => unimplemented!("{other}"),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "value" => self.instance().value().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for Row {}
    impl ListBoxRowImpl for Row {}
    impl PreferencesRowImpl for Row {}
    impl EntryRowImpl for Row {}
}

glib::wrapper! {
    pub(crate) struct Row(ObjectSubclass<imp::Row>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::EntryRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget, gtk::Editable;
}

impl From<&model::Value> for Row {
    fn from(value: &model::Value) -> Self {
        Self::new(value, &gettext("Value"))
    }
}

impl Row {
    pub fn new(value: &model::Value, title: impl Into<String>) -> Self {
        glib::Object::builder::<Self>()
            .property("value", &value)
            .property("title", &title.into())
            .build()
    }

    pub(crate) fn value(&self) -> Option<model::Value> {
        self.imp().value.borrow().to_owned()
    }

    pub(crate) fn set_value(&self, value: Option<model::Value>) {
        if self.value() == value {
            return;
        }

        let imp = self.imp();
        let mut bindings = imp.bindings.borrow_mut();

        while let Some(binding) = bindings.pop() {
            binding.unbind();
        }

        if let Some(ref value) = value {
            let binding = value
                .bind_property("value", self, "text")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
            bindings.push(binding);
        }

        imp.value.replace(value);
        self.notify("value");
    }
}
