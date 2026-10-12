use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PropertyWidgetRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/property_widget_row.ui")]
    pub(crate) struct PropertyWidgetRow {
        #[property(
            get,
            set = |imp: &Self, key: Option<String>| { imp.key.replace(key.unwrap_or_default()); }
        )]
        pub(super) key: RefCell<String>,
        #[property(get, set, nullable)]
        pub(super) widget: RefCell<Option<gtk::Widget>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PropertyWidgetRow {
        const NAME: &'static str = "PdsPropertyWidgetRow";
        type Type = super::PropertyWidgetRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PropertyWidgetRow {
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

    impl WidgetImpl for PropertyWidgetRow {}
    impl ListBoxRowImpl for PropertyWidgetRow {}
    impl PreferencesRowImpl for PropertyWidgetRow {}
    impl ActionRowImpl for PropertyWidgetRow {}
}

glib::wrapper! {
    pub(crate) struct PropertyWidgetRow(ObjectSubclass<imp::PropertyWidgetRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for PropertyWidgetRow {
    fn default() -> Self {
        glib::Object::new()
    }
}
