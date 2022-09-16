use adw::subclass::prelude::ExpanderRowImpl;
use adw::subclass::prelude::PreferencesRowImpl;
use adw::traits::PreferencesRowExt;
use gettextrs::gettext;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container-health-check-log-row.ui")]
    pub(crate) struct ContainerHealthCheckLogRow {
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
    impl ObjectSubclass for ContainerHealthCheckLogRow {
        const NAME: &'static str = "ContainerHealthCheckLogRow";
        type Type = super::ContainerHealthCheckLogRow;
        type ParentType = adw::ExpanderRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerHealthCheckLogRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "log",
                    "Log",
                    "The container health check log",
                    model::HealthCheckLog::static_type(),
                    glib::ParamFlags::READWRITE
                        | glib::ParamFlags::CONSTRUCT
                        | glib::ParamFlags::EXPLICIT_NOTIFY,
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
                "log" => obj.set_log(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "log" => obj.log().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            self.output_text_view.remove_css_class("view");
        }
    }

    impl WidgetImpl for ContainerHealthCheckLogRow {}
    impl ListBoxRowImpl for ContainerHealthCheckLogRow {}
    impl PreferencesRowImpl for ContainerHealthCheckLogRow {}
    impl ExpanderRowImpl for ContainerHealthCheckLogRow {}
}

glib::wrapper! {
    pub(crate) struct ContainerHealthCheckLogRow(ObjectSubclass<imp::ContainerHealthCheckLogRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ExpanderRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::HealthCheckLog> for ContainerHealthCheckLogRow {
    fn from(log: &model::HealthCheckLog) -> Self {
        glib::Object::new(&[("log", &log)]).expect("Failed to create ContainerHealthCheckLogRow")
    }
}

impl ContainerHealthCheckLogRow {
    pub(crate) fn log(&self) -> Option<model::HealthCheckLog> {
        self.imp().log.upgrade()
    }

    pub(crate) fn set_log(&self, value: Option<&model::HealthCheckLog>) {
        if self.log().as_ref() == value {
            return;
        }

        let imp = self.imp();

        match value {
            Some(log) => {
                if log.exit_code() == 0 {
                    imp.exit_code_image.set_icon_name(Some("success-symbolic"));
                    imp.exit_code_image.add_css_class("success");
                    self.set_title(&gettext("Passed Health Run"));
                } else {
                    imp.exit_code_image.set_icon_name(Some("error-symbolic"));
                    imp.exit_code_image.add_css_class("error");
                    self.set_title(&gettext!("Failed Health Run: {}", log.exit_code()));
                }
                imp.start_label.set_label(
                    &glib::DateTime::from_iso8601(log.start(), None)
                        .unwrap()
                        .format(
                            // Translators: This is a date time format (https://valadoc.org/glib-2.0/GLib.DateTime.format.html)
                            &gettext("%x %X"),
                        )
                        .unwrap(),
                );
                imp.output_text_buffer.set_text(log.output());
            }
            None => {
                imp.exit_code_image.set_icon_name(None);
                imp.exit_code_image.remove_css_class("success");
                imp.exit_code_image.remove_css_class("error");
                imp.start_label.set_label("");
                imp.output_text_buffer.set_text("");
            }
        }

        imp.log.set(value);
        self.notify("env-var");
    }
}
