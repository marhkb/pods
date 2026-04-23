use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::IdOrNameRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/id_or_name_row.ui")]
    pub(crate) struct IdOrNameRow {
        #[property(get, set)]
        pub(super) id_or_name: RefCell<String>,
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for IdOrNameRow {
        const NAME: &'static str = "PdsIdOrNameRow";
        type Type = super::IdOrNameRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for IdOrNameRow {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }

    impl WidgetImpl for IdOrNameRow {}

    #[gtk::template_callbacks]
    impl IdOrNameRow {
        #[template_callback]
        fn on_notify_id_or_name(&self) {
            let id_or_name = &*self.id_or_name.borrow();

            let id_or_name = match utils::as_id(id_or_name.as_str()) {
                Some(id) => {
                    self.label.add_css_class("monospace");
                    utils::format_id(id)
                }
                None => {
                    self.label.remove_css_class("monospace");
                    id_or_name.as_str()
                }
            };

            self.label.set_label(id_or_name);
        }
    }
}

glib::wrapper! {
    pub(crate) struct IdOrNameRow(ObjectSubclass<imp::IdOrNameRow>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
