use adw::subclass::prelude::ExpanderRowImpl;
use adw::subclass::prelude::PreferencesRowImpl;
use adw::traits::PreferencesRowExt;
use gettextrs::gettext;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::Row)]
    #[template(resource = "/com/github/marhkb/Pods/ui/health-check-log/row.ui")]
    pub(crate) struct Row {
        #[property(get, set = Self::set_log, explicit_notify, nullable)]
        pub(super) log: glib::WeakRef<model::HealthCheckLog>,
        #[template_child]
        pub(super) exit_code_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) output_text_view: TemplateChild<gtk::TextView>,
        #[template_child]
        pub(super) output_text_buffer: TemplateChild<gtk::TextBuffer>,
        #[template_child]
        pub(super) start_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsHealthCheckLogRow";
        type Type = super::Row;
        type ParentType = adw::ExpanderRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Row {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.output_text_view.remove_css_class("view");
        }
    }

    impl WidgetImpl for Row {}
    impl ListBoxRowImpl for Row {}
    impl PreferencesRowImpl for Row {}
    impl ExpanderRowImpl for Row {}

    impl Row {
        pub(super) fn set_log(&self, value: Option<&model::HealthCheckLog>) {
            let obj = &*self.obj();
            if obj.log().as_ref() == value {
                return;
            }

            match value {
                Some(log) => {
                    if log.exit_code() == 0 {
                        self.exit_code_image.set_icon_name(Some("success-symbolic"));
                        self.exit_code_image.add_css_class("success");
                        obj.set_title(&gettext("Passed Health Run"));
                    } else {
                        self.exit_code_image.set_icon_name(Some("error-symbolic"));
                        self.exit_code_image.add_css_class("error");
                        obj.set_title(&gettext!("Failed Health Run: {}", log.exit_code()));
                    }
                    self.start_label.set_label(
                        &glib::DateTime::from_iso8601(&log.start(), None)
                            .unwrap()
                            .format(
                                // Translators: This is a date time format (https://valadoc.org/glib-2.0/GLib.DateTime.format.html)
                                &gettext("%x %X"),
                            )
                            .unwrap(),
                    );
                    self.output_text_buffer.set_text(&log.output());
                }
                None => {
                    self.exit_code_image.set_icon_name(None);
                    self.exit_code_image.remove_css_class("success");
                    self.exit_code_image.remove_css_class("error");
                    self.start_label.set_label("");
                    self.output_text_buffer.set_text("");
                }
            }

            self.log.set(value);
            obj.notify("log");
        }
    }
}

glib::wrapper! {
    pub(crate) struct Row(ObjectSubclass<imp::Row>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ExpanderRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::HealthCheckLog> for Row {
    fn from(log: &model::HealthCheckLog) -> Self {
        glib::Object::builder().property("log", log).build()
    }
}
