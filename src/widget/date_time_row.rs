use std::cell::Cell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::utils;

#[derive(Clone, Copy, Debug, Default)]
enum TimeFormat {
    Hours12,
    #[default]
    Hours24,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::DateTimeRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/date_time_row.ui")]
    pub(crate) struct DateTimeRow {
        pub(super) desktop_settings: utils::DesktopSettings,
        pub(super) time_format: Cell<TimeFormat>,
        #[property(get, set = Self::set_timestamp, explicit_notify)]
        pub(super) timestamp: Cell<i64>,
        #[template_child]
        pub(super) date_time_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) calendar: TemplateChild<gtk::Calendar>,
        #[template_child]
        pub(super) hour_spin_button: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub(super) hour_adjustment: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub(super) minute_spin_button: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub(super) period_drop_down: TemplateChild<gtk::DropDown>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DateTimeRow {
        const NAME: &'static str = "PdsDateTimeRow";
        type Type = super::DateTimeRow;
        type ParentType = adw::ExpanderRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DateTimeRow {
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

            let obj = &*self.obj();

            obj.load_time_format();
            self.desktop_settings.connect_changed(
                Some("clock-format"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.load_time_format();
                    }
                ),
            );

            obj.bind_property("timestamp", &*self.calendar, "date")
                .bidirectional()
                .transform_to(|_, timestamp| utils::date_time_from_unix_local(timestamp))
                .transform_from(|binding, _: glib::DateTime| source_to_timestamp(binding))
                .build();

            obj.bind_property("timestamp", &*self.hour_spin_button, "value")
                .bidirectional()
                .transform_to(|binding, timestamp| {
                    utils::date_time_from_unix_local(timestamp)
                        .map(|date_time| date_time.hour())
                        .and_then(|hour| {
                            binding
                                .source()
                                .and_then(|source| source.downcast::<Self::Type>().ok())
                                .map(|obj| {
                                    let imp = obj.imp();

                                    match imp.time_format.get() {
                                        TimeFormat::Hours12 if hour > 12 => hour - 12,
                                        _ => hour,
                                    }
                                })
                        })
                        .map(|hour| hour as f64)
                })
                .transform_from(|binding, _: f64| source_to_timestamp(binding))
                .build();

            obj.bind_property("timestamp", &*self.minute_spin_button, "value")
                .bidirectional()
                .transform_to(|_, timestamp| {
                    utils::date_time_from_unix_local(timestamp)
                        .map(|date_time| date_time.minute() as f64)
                })
                .transform_from(|binding, _: f64| source_to_timestamp(binding))
                .build();

            obj.bind_property("timestamp", &*self.period_drop_down, "selected")
                .bidirectional()
                .transform_to(|_, timestamp: i64| {
                    utils::date_time_from_unix_local(timestamp)
                        .map(|date_time| if date_time.hour() < 12 { 0_u32 } else { 1_u32 })
                })
                .transform_from(|binding, _: u32| source_to_timestamp(binding))
                .build();

            Self::Type::this_expression("timestamp")
                .chain_closure::<String>(closure!(|_: Self::Type, unix: i64| {
                    utils::date_time_from_unix_local(unix)
                        .and_then(|date_time| {
                            date_time
                                .format(
                                    // Translators: This is a date time format (https://valadoc.org/glib-2.0/GLib.DateTime.format.html)
                                    &gettext("%x %X"),
                                )
                                .ok()
                        })
                        .map(|formatted| formatted.to_string())
                        .unwrap_or_else(|| gettext("Invalid date format"))
                }))
                .bind(&*self.date_time_label, "label", Some(obj));

            obj.set_timestamp(glib::DateTime::now_local().unwrap().to_unix());
        }
    }

    impl WidgetImpl for DateTimeRow {}
    impl ListBoxRowImpl for DateTimeRow {}
    impl PreferencesRowImpl for DateTimeRow {}
    impl ExpanderRowImpl for DateTimeRow {}

    #[gtk::template_callbacks]
    impl DateTimeRow {
        fn set_timestamp(&self, timestamp: i64) {
            // remove seconds
            let timestamp = (timestamp / 60) * 60;
            if self.timestamp.get() == timestamp {
                return;
            }

            self.timestamp.set(timestamp);
            self.obj().notify_timestamp();
        }

        #[template_callback]
        fn on_spin_button_output(spin_button: &gtk::SpinButton) -> glib::Propagation {
            spin_button.set_text(&format!("{:02}", spin_button.value()));
            glib::Propagation::Stop
        }
    }
}

glib::wrapper! {
    pub(crate) struct DateTimeRow(ObjectSubclass<imp::DateTimeRow>)
        @extends gtk::Widget, gtk::ListBox, adw::PreferencesRow, adw::ExpanderRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget, gtk::ListBoxRow;
}

impl DateTimeRow {
    fn load_time_format(&self) {
        let imp = self.imp();

        match imp.desktop_settings.get::<String>("clock-format").as_str() {
            "12h" => {
                imp.hour_adjustment.set_lower(1.0);
                imp.hour_adjustment.set_upper(12.0);
                imp.period_drop_down.set_visible(true);
                imp.time_format.set(TimeFormat::Hours12);
            }
            other => {
                if other != "24h" {
                    log::warn!("Unknown time format '{other}'. Falling back to '24h'.");
                }
                imp.hour_adjustment.set_lower(0.0);
                imp.hour_adjustment.set_upper(23.0);
                imp.period_drop_down.set_visible(false);
                imp.time_format.set(TimeFormat::Hours24);
            }
        }
    }
}

fn source_to_timestamp(binding: &glib::Binding) -> Option<i64> {
    binding
        .source()
        .and_then(|obj| obj.downcast::<DateTimeRow>().ok())
        .map(|obj| {
            let imp = obj.imp();

            let date = imp.calendar.date();

            glib::DateTime::from_local(
                date.year(),
                date.month(),
                date.day_of_month(),
                {
                    let hour = imp.hour_spin_button.value_as_int();
                    match imp.time_format.get() {
                        TimeFormat::Hours12
                            if imp.period_drop_down.selected() == 1 && hour < 12 =>
                        {
                            hour + 12
                        }
                        _ => hour,
                    }
                },
                imp.minute_spin_button.value_as_int(),
                0.0,
            )
            .unwrap()
            .to_unix()
        })
}
