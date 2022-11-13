use adw::subclass::prelude::ActionRowImpl;
use adw::subclass::prelude::PreferencesRowImpl;
use adw::traits::PreferencesRowExt;
use gtk::glib;
use gtk::glib::clone;
use gtk::pango;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/component/property-row.ui")]
    pub(crate) struct PropertyRow {
        #[template_child]
        pub(super) value_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PropertyRow {
        const NAME: &'static str = "PdsPropertyRow";
        type Type = super::PropertyRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PropertyRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::builder("key")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecString::builder("value")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecEnum::builder::<pango::WrapMode>(
                        "value-wrap-mode",
                        pango::WrapMode::Word,
                    )
                    .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                    .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "key" => obj.set_key(value.get().unwrap_or_default()),
                "value" => obj.set_value(value.get().unwrap_or_default()),
                "value-wrap-mode" => obj.set_value_wrap_mode(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "key" => obj.key().to_value(),
                "value" => obj.value().to_value(),
                "value-wrap-mode" => obj.value_wrap_mode().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            obj.connect_notify_local(
                Some("title"),
                clone!(@weak obj => move |_, _| obj.notify("key")),
            );
            self.value_label.connect_notify_local(
                Some("label"),
                clone!(@weak obj => move |_, _| obj.notify("value")),
            );
            self.value_label.connect_notify_local(
                Some("wrap-mode"),
                clone!(@weak obj => move |_, _| obj.notify("value-wrap-mode")),
            );
        }
    }

    impl WidgetImpl for PropertyRow {}
    impl ListBoxRowImpl for PropertyRow {}
    impl PreferencesRowImpl for PropertyRow {}
    impl ActionRowImpl for PropertyRow {}
}

glib::wrapper! {
    pub(crate) struct PropertyRow(ObjectSubclass<imp::PropertyRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl Default for PropertyRow {
    fn default() -> Self {
        glib::Object::builder::<Self>().build()
    }
}

impl PropertyRow {
    pub(crate) fn new(key: &str, value: &str) -> Self {
        glib::Object::builder::<Self>()
            .property("key", key)
            .property("value", value)
            .build()
    }

    pub(crate) fn key(&self) -> glib::GString {
        self.title()
    }

    pub(crate) fn set_key(&self, key: &str) {
        if key == self.key().as_str() {
            return;
        }
        self.set_title(key);
    }

    pub(crate) fn value(&self) -> glib::GString {
        self.imp().value_label.label()
    }

    pub(crate) fn set_value(&self, value: &str) {
        self.style_context().color();
        if value == self.value().as_str() {
            return;
        }
        self.imp().value_label.set_label(value);
    }

    pub(crate) fn value_wrap_mode(&self) -> pango::WrapMode {
        self.imp().value_label.wrap_mode()
    }

    pub(crate) fn set_value_wrap_mode(&self, mode: pango::WrapMode) {
        if mode == self.value_wrap_mode() {
            return;
        }
        self.imp().value_label.set_wrap_mode(mode);
    }
}
