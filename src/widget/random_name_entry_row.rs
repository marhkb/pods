use std::cell::Cell;
use std::cell::RefCell;
use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::utils;
mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::RandomNameEntryRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/random_name_entry_row.ui")]
    pub(crate) struct RandomNameEntryRow {
        #[property(get, set)]
        pub(super) blank: Cell<bool>,
        #[template_child]
        pub(super) generate_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RandomNameEntryRow {
        const NAME: &'static str = "PdsRandomNameEntryRow";
        type Type = super::RandomNameEntryRow;
        type ParentType = adw::EntryRow;
        type Interfaces = (gtk::Editable,);

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(
                "random-name-entry-row.generate",
                None,
                move |widget, _, _| {
                    widget.generate_random_name();
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RandomNameEntryRow {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "text" => self.obj().set_text(value.get().unwrap()),
                _ => self.derived_set_property(id, value, pspec),
            }
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "text" => self.obj().text().to_value(),
                _ => self.derived_property(id, pspec),
            }
        }
    }

    impl WidgetImpl for RandomNameEntryRow {
        fn realize(&self) {
            self.parent_realize();

            let obj = &*self.obj();
            if !obj.blank() {
                obj.generate_random_name();
            }
        }
    }
    impl ListBoxRowImpl for RandomNameEntryRow {}
    impl PreferencesRowImpl for RandomNameEntryRow {}
    impl EntryRowImpl for RandomNameEntryRow {}
    impl EditableImpl for RandomNameEntryRow {}
}

glib::wrapper! {
    pub(crate) struct RandomNameEntryRow(ObjectSubclass<imp::RandomNameEntryRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::EntryRow,
        @implements gtk::Editable;
}

impl Default for RandomNameEntryRow {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl RandomNameEntryRow {
    pub(crate) fn generate_random_name(&self) {
        self.set_text(&utils::NAME_GENERATOR.borrow_mut().next().unwrap());
    }
}
