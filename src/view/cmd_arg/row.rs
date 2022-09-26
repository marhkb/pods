use std::cell::RefCell;

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/cmd-arg/row.ui")]
    pub(crate) struct Row {
        pub(super) arg: RefCell<Option<model::CmdArg>>,
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsCmdArgRow";
        type Type = super::Row;
        type ParentType = adw::EntryRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("arg.remove", None, |widget, _, _| {
                if let Some(cmd_arg) = widget.cmd_arg() {
                    cmd_arg.remove_request();
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
                    "arg",
                    "Arg",
                    "The command argument",
                    model::CmdArg::static_type(),
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
                "arg" => obj.set_cmd_arg(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "arg" => obj.cmd_arg().to_value(),
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

impl From<&model::CmdArg> for Row {
    fn from(cmd_arg: &model::CmdArg) -> Self {
        glib::Object::new(&[("arg", &cmd_arg)]).expect("Failed to create PdsCmdArgRow")
    }
}

impl Row {
    pub(crate) fn cmd_arg(&self) -> Option<model::CmdArg> {
        self.imp().arg.borrow().to_owned()
    }

    pub(crate) fn set_cmd_arg(&self, value: Option<model::CmdArg>) {
        if self.cmd_arg() == value {
            return;
        }

        let imp = self.imp();
        let mut bindings = imp.bindings.borrow_mut();

        while let Some(binding) = bindings.pop() {
            binding.unbind();
        }

        if let Some(ref cmd_arg) = value {
            let binding = cmd_arg
                .bind_property("arg", self, "text")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
            bindings.push(binding);
        }

        imp.arg.replace(value);
        self.notify("arg");
    }
}
