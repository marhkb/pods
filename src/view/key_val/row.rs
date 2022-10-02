use std::cell::RefCell;

use gtk::glib;
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
                vec![glib::ParamSpecObject::new(
                    "key-val",
                    "Key Value",
                    "The underlying key-value pair",
                    model::KeyVal::static_type(),
                    glib::ParamFlags::READWRITE
                        | glib::ParamFlags::CONSTRUCT
                        | glib::ParamFlags::EXPLICIT_NOTIFY,
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
                "key-val" => obj.set_key_val(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "key-val" => obj.key_val().to_value(),
                _ => unimplemented!(),
            }
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
        glib::Object::new(&[("key-val", &key_val)]).expect("Failed to create PdsKeyValRow")
    }
}

impl Row {
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
}
