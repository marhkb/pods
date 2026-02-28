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
        #[property(get, set)]
        pub(super) prune_until_timestamp: Cell<i64>,
        #[template_child]
        pub(super) prune_until_label: TemplateChild<gtk::Label>,
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

            gtk::ClosureExpression::new::<i64>(
                [
                    self.calendar.property_expression("year"),
                    self.calendar.property_expression("month"),
                    self.calendar.property_expression("day"),
                    self.hour_spin_button.property_expression("value"),
                    self.minute_spin_button.property_expression("value"),
                    self.period_drop_down.property_expression("selected"),
                ],
                closure!(|obj: Self::Type,
                          year: i32,
                          month: i32,
                          day: i32,
                          hour: f64,
                          minute: f64,
                          period: u32| {
                    glib::DateTime::from_local(
                        year,
                        month + 1,
                        day,
                        if matches!(obj.imp().time_format.get(), TimeFormat::Hours12)
                            && period == 1
                            && hour < 12.0
                        {
                            hour as i32 + 12
                        } else {
                            hour as i32
                        },
                        minute as i32,
                        0.0,
                    )
                    .unwrap()
                    .to_unix()
                }),
            )
            .bind(obj, "prune-until-timestamp", Some(obj));

            Self::Type::this_expression("prune-until-timestamp")
                .chain_closure::<String>(closure!(|_: Self::Type, unix: i64| {
                    utils::date_time_from_unix_local(unix)
                        .and_then(|date_time| {
                            date_time
                                .format(
                                    // Translators: This is a date time format (https://valadoc.org/glib-2.0/GLib.DateTime.format.html)
                                    &gettext("%x %H:%M %p"),
                                )
                                .ok()
                        })
                        .map(|formatted| formatted.to_string())
                        .unwrap_or_else(|| gettext("Invalid date format"))
                }))
                .bind(&*self.prune_until_label, "label", Some(obj));

            let (hour, minute) = glib::DateTime::now_local()
                .map(|now| (now.hour(), now.minute()))
                .unwrap_or((0, 0));

            self.hour_spin_button.set_value(hour as f64);
            self.minute_spin_button.set_value(minute as f64);
            self.period_drop_down.set_selected(u32::from(hour >= 12));
        }
    }

    impl WidgetImpl for DateTimeRow {}
    impl ListBoxRowImpl for DateTimeRow {}
    impl PreferencesRowImpl for DateTimeRow {}
    impl ExpanderRowImpl for DateTimeRow {}

    #[gtk::template_callbacks]
    impl DateTimeRow {
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
                imp.hour_adjustment.set_upper(11.0);
                imp.period_drop_down.set_visible(true);
                imp.time_format.set(TimeFormat::Hours12);
            }
            other => {
                if other != "24h" {
                    log::warn!("Unknown time format '{other}'. Falling back to '24h'.");
                }
                imp.hour_adjustment.set_upper(23.0);
                imp.period_drop_down.set_visible(false);
                imp.time_format.set(TimeFormat::Hours24);
            }
        }
    }
}
