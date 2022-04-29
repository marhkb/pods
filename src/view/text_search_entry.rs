use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/text-search-entry.ui")]
    pub(crate) struct TextSearchEntry {
        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) text: TemplateChild<gtk::Text>,
        #[template_child]
        pub(super) info_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TextSearchEntry {
        const NAME: &'static str = "TextSearchEntry";
        type Type = super::TextSearchEntry;
        type ParentType = gtk::Widget;
        type Interfaces = (gtk::Editable,);

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.set_css_name("entry");
            klass.set_accessible_role(gtk::AccessibleRole::TextBox);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TextSearchEntry {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecString::new(
                    "info",
                    "Info",
                    "The info label of this TextSearchEntry",
                    Option::default(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
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
                "info" => obj.set_info(value.get().unwrap()),
                property => self
                    .text
                    .try_set_property_from_value(property, value)
                    .unwrap(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "info" => obj.info().to_value(),
                property => self.text.try_property_value(property).unwrap(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            self.text.connect_notify_local(
                None,
                clone!(@weak obj => move |_, pspec| if obj.has_property(pspec.name(), None) {
                    obj.notify(pspec.name())
                }),
            );

            self.info_label.connect_notify_local(
                Some("label"),
                clone!(@weak obj => move |_, _| obj.notify("info")),
            );
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.image.unparent();
            self.text.unparent();
            self.info_label.unparent();
        }
    }

    impl WidgetImpl for TextSearchEntry {
        fn grab_focus(&self, _widget: &Self::Type) -> bool {
            self.text.grab_focus()
        }
    }

    impl EditableImpl for TextSearchEntry {
        fn delegate(&self, _editable: &Self::Type) -> Option<gtk::Editable> {
            Some(self.text.clone().upcast())
        }
    }
}

glib::wrapper! {
    pub(crate) struct TextSearchEntry(ObjectSubclass<imp::TextSearchEntry>)
        @extends gtk::Widget,
        @implements gtk::Editable;
}

impl TextSearchEntry {
    pub(crate) fn info(&self) -> glib::GString {
        self.imp().info_label.label()
    }

    pub(crate) fn set_info(&self, value: &str) {
        if self.info().as_str() == value {
            return;
        }
        self.imp().info_label.set_label(value);
    }
}
