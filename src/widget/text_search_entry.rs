use std::cell::Cell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::utils;

const LAYOUT_SPACING: i32 = 6;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::TextSearchEntry)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/text_search_entry.ui")]
    pub(crate) struct TextSearchEntry {
        #[property(get, set)]
        pub(super) case_sensitive: Cell<bool>,
        #[property(get, set)]
        pub(super) info: RefCell<String>,
        #[property(get, set)]
        pub(super) regex: Cell<bool>,
        #[property(get, set)]
        pub(super) whole_word: Cell<bool>,

        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) text: TemplateChild<gtk::Text>,
        #[template_child]
        pub(super) info_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) options_box: TemplateChild<gtk::Box>,
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
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            if id <= Self::derived_properties().len() {
                self.derived_set_property(id, value, pspec);
            } else {
                self.delegate_set_property(id, value, pspec);
            }
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            if id <= Self::derived_properties().len() {
                self.derived_property(id, pspec)
            } else {
                self.delegate_get_property(id, pspec).unwrap()
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let settings = utils::PodsSettings::default();
            settings.bind("search-regex", obj, "regex").build();
            settings
                .bind("search-case-sensitive", obj, "case-sensitive")
                .build();
            settings
                .bind("search-whole-word", obj, "whole-word")
                .build();
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
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
            if obj.has_property(pspec.name()) {
                obj.notify(pspec.name());
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct TextSearchEntry(ObjectSubclass<imp::TextSearchEntry>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Editable;
}
