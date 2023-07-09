use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::utils;

const LAYOUT_SPACING: i32 = 6;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/text_search_entry.ui")]
    pub(crate) struct TextSearchEntry {
        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) text: TemplateChild<gtk::Text>,
        #[template_child]
        pub(super) info_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) options_box: TemplateChild<gtk::Box>,
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
            klass.bind_template();
            klass.bind_template_callbacks();

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
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecBoolean::builder("regex")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecBoolean::builder("case-sensitive")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecBoolean::builder("whole-word")
                        .explicit_notify()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "info" => obj.set_info(value.get().unwrap()),
                "regex" => obj.set_regex(value.get().unwrap()),
                "case-sensitive" => obj.set_case_sensitive(value.get().unwrap()),
                "whole-word" => obj.set_whole_word(value.get().unwrap()),
                property => self.text.set_property_from_value(property, value),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
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

            let settings = utils::PodsSettings::default();
            settings
                .bind("search-regex", &self.regex_button.get(), "active")
                .build();
            settings
                .bind("search-case-sensitive", &self.case_button.get(), "active")
                .build();
            settings
                .bind("search-whole-word", &self.word_button.get(), "active")
                .build();
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for TextSearchEntry {
        fn grab_focus(&self) -> bool {
            self.text.grab_focus()
        }

        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::WidthForHeight
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            if orientation == gtk::Orientation::Horizontal {
                let (min_image, nat_image, _, _) = self.image.measure(orientation, for_size);
                let (min_text, nat_text, _, _) = self.text.measure(orientation, for_size);
                let (min_options_box, nat_options_box, _, _) =
                    self.options_box.measure(orientation, for_size);

                (
                    LAYOUT_SPACING * 3 + min_image + min_options_box + min_text + /* assume info label width */ 37,
                    nat_image + nat_options_box + nat_text,
                    -1,
                    -1,
                )
            } else {
                self.parent_measure(orientation, for_size)
            }
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            let image_width = self.image.preferred_size().1.width();
            let info_label_width = self.info_label.preferred_size().1.width();
            let options_box_width = self.options_box.preferred_size().1.width();

            let text_width =
                width - image_width - info_label_width - options_box_width - LAYOUT_SPACING * 3;

            self.image
                .size_allocate(&gtk::Allocation::new(0, 0, image_width, height), baseline);

            self.text.size_allocate(
                &gtk::Allocation::new(image_width + LAYOUT_SPACING, 0, text_width, height),
                baseline,
            );

            self.info_label.size_allocate(
                &gtk::Allocation::new(
                    image_width + text_width + LAYOUT_SPACING * 2,
                    0,
                    info_label_width,
                    height,
                ),
                baseline,
            );

            self.options_box.size_allocate(
                &gtk::Allocation::new(
                    image_width + text_width + info_label_width + LAYOUT_SPACING * 3,
                    0,
                    options_box_width,
                    height,
                ),
                baseline,
            );
        }
    }

    impl EditableImpl for TextSearchEntry {
        fn delegate(&self) -> Option<gtk::Editable> {
            Some(self.text.clone().upcast())
        }
    }

    #[gtk::template_callbacks]
    impl TextSearchEntry {
        #[template_callback]
        fn on_text_notify(&self, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            if obj.has_property(pspec.name(), None) {
                obj.notify(pspec.name());
            }
        }

        #[template_callback]
        fn on_info_label_notify_label(&self) {
            self.obj().notify("info");
        }

        #[template_callback]
        fn on_regex_button_notify_active(&self) {
            self.obj().notify("regex");
        }

        #[template_callback]
        fn on_case_button_notify_active(&self) {
            self.obj().notify("case-sensitive");
        }

        #[template_callback]
        fn on_word_button_notify_active(&self) {
            self.obj().notify("whole-word");
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
