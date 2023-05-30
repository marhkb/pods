use std::cell::RefCell;

use gettextrs::gettext;
use glib::clone;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::OnceCell as SyncOnceCell;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::KeyValRow)]
    #[template(file = "key_val_row.ui")]
    pub(crate) struct KeyValRow {
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[property(get, set = Self::set_key_val, construct)]
        pub(super) key_val: RefCell<Option<model::KeyVal>>,
        #[template_child]
        pub(super) key_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) value_entry: TemplateChild<gtk::Entry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for KeyValRow {
        const NAME: &'static str = "PdsKeyValRow";
        type Type = super::KeyValRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
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

    impl ObjectImpl for KeyValRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: SyncOnceCell<Vec<glib::ParamSpec>> = SyncOnceCell::new();
            PROPERTIES.get_or_init(|| {
                Self::derived_properties()
                    .iter()
                    .cloned()
                    .chain(vec![
                        glib::ParamSpecString::builder("key-placeholder-text")
                            .construct()
                            .explicit_notify()
                            .build(),
                        glib::ParamSpecString::builder("value-placeholder-text")
                            .construct()
                            .explicit_notify()
                            .build(),
                    ])
                    .collect::<Vec<_>>()
            })
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "key-placeholder-text" => {
                    self.obj()
                        .set_key_placeholder_text(value.get().unwrap_or_default());
                }
                "value-placeholder-text" => {
                    self.obj()
                        .set_value_placeholder_text(value.get().unwrap_or_default());
                }
                _ => self.derived_set_property(id, value, pspec),
            }
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "key-placeholder-text" => self.obj().key_placeholder_text().to_value(),
                "value-placeholder-text" => self.obj().value_placeholder_text().to_value(),
                _ => self.derived_property(id, pspec),
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

    impl WidgetImpl for KeyValRow {}
    impl ListBoxRowImpl for KeyValRow {}

    impl KeyValRow {
        pub(super) fn set_key_val(&self, value: Option<model::KeyVal>) {
            let obj = &*self.obj();
            if obj.key_val() == value {
                return;
            }

            let mut bindings = self.bindings.borrow_mut();

            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }

            if let Some(ref key_val) = value {
                let binding = key_val
                    .bind_property("key", &*self.key_entry, "text")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);

                let binding = key_val
                    .bind_property("value", &*self.value_entry, "text")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);
            }

            self.key_val.replace(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct KeyValRow(ObjectSubclass<imp::KeyValRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::KeyVal> for KeyValRow {
    fn from(key_val: &model::KeyVal) -> Self {
        KeyValRow::new(&gettext("Key"), &gettext("Value"), key_val)
    }
}

impl KeyValRow {
    pub fn new(
        key_placeholder_text: &str,
        value_placeholder_text: &str,
        entry: &model::KeyVal,
    ) -> Self {
        glib::Object::builder()
            .property("key-val", entry)
            .property("key-placeholder-text", key_placeholder_text)
            .property("value-placeholder-text", value_placeholder_text)
            .build()
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
