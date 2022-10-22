use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/component/text-search-entry.ui")]
    pub(crate) struct TextSearchEntry {
        #[template_child]
        pub(super) text: TemplateChild<gtk::Text>,
        #[template_child]
        pub(super) info_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) regex_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) case_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) word_button: TemplateChild<gtk::ToggleButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TextSearchEntry {
        const NAME: &'static str = "PdsTextSearchEntry";
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
                vec![
                    glib::ParamSpecString::builder("info")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecBoolean::builder("regex")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecBoolean::builder("case-sensitive")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecBoolean::builder("whole-word")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.instance();
            match pspec.name() {
                "info" => obj.set_info(value.get().unwrap()),
                "regex" => obj.set_regex(value.get().unwrap()),
                "case-sensitive" => obj.set_case_sensitive(value.get().unwrap()),
                "whole-word" => obj.set_whole_word(value.get().unwrap()),
                property => self.text.set_property_from_value(property, value),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.instance();
            match pspec.name() {
                "info" => obj.info().to_value(),
                "regex" => obj.is_regex().to_value(),
                "case-sensitive" => obj.is_case_sensitive().to_value(),
                "whole-word" => obj.is_whole_word().to_value(),
                property => self.text.property_value(property),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            let settings = utils::PodsSettings::default();
            settings
                .bind("search-regex", &*self.regex_button, "active")
                .build();
            settings
                .bind("search-case-sensitive", &*self.case_button, "active")
                .build();
            settings
                .bind("search-whole-word", &*self.word_button, "active")
                .build();

            self.text.connect_notify_local(
                None,
                clone!(@weak obj => move |_, pspec| if obj.has_property(pspec.name(), None) {
                    obj.notify(pspec.name())
                }),
            );

            self.info_label
                .connect_label_notify(clone!(@weak obj => move |_| obj.notify("info")));

            self.regex_button
                .connect_active_notify(clone!(@weak obj => move |_| obj.notify("regex")));

            self.case_button
                .connect_active_notify(clone!(@weak obj => move |_| obj.notify("case-sensitive")));

            self.word_button
                .connect_active_notify(clone!(@weak obj => move |_| obj.notify("whole-word")));
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.instance()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for TextSearchEntry {
        fn grab_focus(&self) -> bool {
            self.text.grab_focus()
        }
    }

    impl EditableImpl for TextSearchEntry {
        fn delegate(&self) -> Option<gtk::Editable> {
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

    pub(crate) fn is_regex(&self) -> bool {
        self.imp().regex_button.is_active()
    }

    pub(crate) fn set_regex(&self, value: bool) {
        self.imp().regex_button.set_active(value);
    }

    pub(crate) fn is_case_sensitive(&self) -> bool {
        self.imp().case_button.is_active()
    }

    pub(crate) fn set_case_sensitive(&self, value: bool) {
        self.imp().case_button.set_active(value);
    }

    pub(crate) fn is_whole_word(&self) -> bool {
        self.imp().word_button.is_active()
    }

    pub(crate) fn set_whole_word(&self, value: bool) {
        self.imp().word_button.set_active(value);
    }
}
