use std::cell::RefCell;

use adw::subclass::prelude::EntryRowImpl;
use adw::subclass::prelude::PreferencesRowImpl;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::traits::EditableExt;
use gtk::CompositeTemplate;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/random-name-entry-row.ui")]
    pub(crate) struct RandomNameEntryRow {
        pub(super) names: RefCell<names::Generator<'static>>,
        #[template_child]
        pub(super) generate_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RandomNameEntryRow {
        const NAME: &'static str = "RandomNameEntryRow";
        type Type = super::RandomNameEntryRow;
        type ParentType = adw::EntryRow;
        type Interfaces = (gtk::Editable,);

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

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
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.generate_random_name()
        }
    }

    impl WidgetImpl for RandomNameEntryRow {}
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

impl RandomNameEntryRow {
    pub(crate) fn generate_random_name(&self) {
        self.set_text(&self.imp().names.borrow_mut().next().unwrap());
    }
}
