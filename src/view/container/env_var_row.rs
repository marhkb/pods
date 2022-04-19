use std::cell::RefCell;

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use once_cell::sync::Lazy;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/env-var-row.ui")]
    pub(crate) struct EnvVarRow {
        pub(super) env_var: RefCell<Option<model::EnvVar>>,
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[template_child]
        pub(super) key_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) value_entry: TemplateChild<gtk::Entry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EnvVarRow {
        const NAME: &'static str = "EnvVarRow";
        type Type = super::EnvVarRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("env-var.remove", None, |widget, _, _| {
                if let Some(env_var) = widget.env_var() {
                    env_var.remove_request();
                }
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EnvVarRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "env-var",
                    "Env Var",
                    "The underlying environment variable",
                    model::EnvVar::static_type(),
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
                "env-var" => obj.set_env_var(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "env-var" => obj.env_var().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EnvVarRow {}
    impl ListBoxRowImpl for EnvVarRow {}
}

glib::wrapper! {
    pub(crate) struct EnvVarRow(ObjectSubclass<imp::EnvVarRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::EnvVar> for EnvVarRow {
    fn from(env_var: &model::EnvVar) -> Self {
        glib::Object::new(&[("env-var", &env_var)]).expect("Failed to create EnvVarRow")
    }
}

impl EnvVarRow {
    pub(crate) fn env_var(&self) -> Option<model::EnvVar> {
        self.imp().env_var.borrow().to_owned()
    }

    pub(crate) fn set_env_var(&self, value: Option<model::EnvVar>) {
        if self.env_var() == value {
            return;
        }

        let imp = self.imp();
        let mut bindings = imp.bindings.borrow_mut();

        while let Some(binding) = bindings.pop() {
            binding.unbind();
        }

        if let Some(ref env_var) = value {
            let binding = env_var
                .bind_property("key", &*imp.key_entry, "text")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
            bindings.push(binding);

            let binding = env_var
                .bind_property("value", &*imp.value_entry, "text")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
            bindings.push(binding);
        }

        imp.env_var.replace(value);
        self.notify("env-var");
    }
}
