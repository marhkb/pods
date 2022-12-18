use std::cell::RefCell;

use gettextrs::gettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/key-val/row.ui")]
    pub(crate) struct Row {
        pub(super) key_val: RefCell<Option<model::KeyVal>>,
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[template_child]
        pub(super) key_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) value_entry: TemplateChild<gtk::Entry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsKeyValRow";
        type Type = super::Row;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("key-val-row.remove", None, |widget, _, _| {
                if let Some(key_val) = widget.key_val() {
                    key_val.remove_request();
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
                vec![
                    glib::ParamSpecObject::builder::<model::KeyVal>("key-val")
                        .construct()
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecString::builder("key-placeholder-text")
                        .construct()
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecString::builder("value-placeholder-text")
                        .construct()
                        .explicit_notify()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "key-val" => obj.set_key_val(value.get().unwrap_or_default()),
                "key-placeholder-text" => {
                    obj.set_key_placeholder_text(value.get().unwrap_or_default());
                }
                "value-placeholder-text" => {
                    obj.set_value_placeholder_text(value.get().unwrap_or_default());
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "key-val" => obj.key_val().to_value(),
                "key-placeholder-text" => obj.key_placeholder_text().to_value(),
                "value-placeholder-text" => obj.value_placeholder_text().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();
            self.key_entry
                .connect_placeholder_text_notify(clone!(@weak obj => move |_| {
                    obj.notify("key-placeholder-text");
                }));
            self.key_entry
                .connect_placeholder_text_notify(clone!(@weak obj => move |_| {
                    obj.notify("value-placeholder-text");
                }));
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

impl From<&model::KeyVal> for Row {
    fn from(key_val: &model::KeyVal) -> Self {
        Row::new(gettext("Key"), gettext("Value"), key_val)
    }
}

impl Row {
    pub fn new(
        key_placeholder_text: impl Into<String>,
        value_placeholder_text: impl Into<String>,
        entry: &model::KeyVal,
    ) -> Self {
        glib::Object::builder()
            .property("key-val", entry)
            .property("key-placeholder-text", key_placeholder_text.into())
            .property("value-placeholder-text", value_placeholder_text.into())
            .build()
    }
    pub(crate) fn key_val(&self) -> Option<model::KeyVal> {
        self.imp().key_val.borrow().to_owned()
    }

    pub(crate) fn set_key_val(&self, value: Option<model::KeyVal>) {
        if self.key_val() == value {
            return;
        }

        let imp = self.imp();
        let mut bindings = imp.bindings.borrow_mut();

        while let Some(binding) = bindings.pop() {
            binding.unbind();
        }

        if let Some(ref key_val) = value {
            let binding = key_val
                .bind_property("key", &*imp.key_entry, "text")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
            bindings.push(binding);

            let binding = key_val
                .bind_property("value", &*imp.value_entry, "text")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
            bindings.push(binding);
        }

        imp.key_val.replace(value);
        self.notify("key-val");
    }

    pub(crate) fn key_placeholder_text(&self) -> Option<glib::GString> {
        self.imp().key_entry.placeholder_text()
    }

    pub(crate) fn set_key_placeholder_text(&self, value: Option<&str>) {
        self.imp().key_entry.set_placeholder_text(value);
    }

    pub(crate) fn value_placeholder_text(&self) -> Option<glib::GString> {
        self.imp().value_entry.placeholder_text()
    }

    pub(crate) fn set_value_placeholder_text(&self, value: Option<&str>) {
        self.imp().value_entry.set_placeholder_text(value);
    }
}
