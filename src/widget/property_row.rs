use std::cell::RefCell;
use std::marker::PhantomData;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;
use gtk::pango;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PropertyRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/property_row.ui")]
    pub(crate) struct PropertyRow {
        #[property(
            get,
            set = |imp: &Self, key: Option<String>| { imp.key.replace(key.unwrap_or_default()); }
        )]
        pub(super) key: RefCell<String>,
        #[property(
            get,
            set = |imp: &Self, value: Option<String>| { imp.value.replace(value.unwrap_or_default()); }
        )]
        pub(super) value: RefCell<String>,

        #[property(
            get = |imp: &Self| imp.value_label.wrap_mode(),
            set = Self::set_value_wrap_mode,
            explicit_notify,
            builder(pango::WrapMode::Word)
        )]
        _value_wrap_mode: PhantomData<pango::WrapMode>,

        #[template_child]
        pub(super) value_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PropertyRow {
        const NAME: &'static str = "PdsPropertyRow";
        type Type = super::PropertyRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PropertyRow {
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

    impl WidgetImpl for PropertyRow {}
    impl ListBoxRowImpl for PropertyRow {}
    impl PreferencesRowImpl for PropertyRow {}
    impl ActionRowImpl for PropertyRow {}

    #[gtk::template_callbacks]
    impl PropertyRow {
        #[template_callback]
        fn on_value_label_notify_wrap_mode(&self) {
            self.obj().notify_value_wrap_mode();
        }

        fn set_value_wrap_mode(&self, mode: pango::WrapMode) {
            if mode == self.value_label.wrap_mode() {
                return;
            }
            self.value_label.set_wrap_mode(mode);
        }
    }
}

glib::wrapper! {
    pub(crate) struct PropertyRow(ObjectSubclass<imp::PropertyRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl Default for PropertyRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl PropertyRow {
    pub(crate) fn new(key: &str, value: &str) -> Self {
        glib::Object::builder()
            .property("key", key)
            .property("value", value)
            .build()
    }
}
