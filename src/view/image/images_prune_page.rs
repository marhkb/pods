use std::cell::Cell;

use adw::subclass::prelude::*;
use adw::traits::ExpanderRowExt;
use gettextrs::gettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::podman;
use crate::utils;

#[derive(Clone, Copy, Debug, Default)]
enum TimeFormat {
    Hours12,
    #[default]
    Hours24,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/images-prune-page.ui")]
    pub(crate) struct ImagesPrunePage {
        pub(super) desktop_settings: utils::DesktopSettings,
        pub(super) pods_settings: utils::PodsSettings,
        pub(super) time_format: Cell<TimeFormat>,
        pub(super) prune_until_timestamp: Cell<i64>,
        pub(super) client: WeakRef<model::Client>,
        #[template_child]
        pub(super) header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub(super) button_prune: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) preferences_page: TemplateChild<adw::PreferencesPage>,
        #[template_child]
        pub(super) prune_all_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) prune_external_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) prune_until_expander_row: TemplateChild<adw::ExpanderRow>,
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
        pub(super) period_combo_box: TemplateChild<gtk::ComboBoxText>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagesPrunePage {
        const NAME: &'static str = "ImagesPrunePage";
        type Type = super::ImagesPrunePage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("images.prune", None, |widget, _, _| {
                widget.prune();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagesPrunePage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "client",
                        "Client",
                        "The client",
                        model::Client::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecInt64::new(
                        "prune-until-timestamp",
                        "Prune Until Timestamp",
                        "Images created before this timestamp are pruned",
                        0,
                        i64::MAX,
                        0,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
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
                "client" => self.client.set(value.get().unwrap()),
                "prune-until-timestamp" => obj.set_prune_until_timestamp(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "client" => obj.client().to_value(),
                "prune-until-timestamp" => obj.prune_until_timestamp().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let client = obj.client().unwrap();
            obj.action_set_enabled("images.prune", !client.pruning());
            client.connect_notify_local(
                Some("pruning"),
                clone!(@weak obj => move |client, _| {
                    obj.action_set_enabled("images.prune", !client.pruning());
                }),
            );

            obj.load_time_format();
            self.desktop_settings.connect_changed(
                Some("clock-format"),
                clone!(@weak obj => move |_, _| {
                    obj.load_time_format();
                }),
            );

            setup_time_spin_button(&*self.hour_spin_button);
            setup_time_spin_button(&*self.minute_spin_button);

            gtk::ClosureExpression::new::<i64, _, _>(
                [
                    self.calendar.property_expression("year"),
                    self.calendar.property_expression("month"),
                    self.calendar.property_expression("day"),
                    self.hour_spin_button.property_expression("value"),
                    self.minute_spin_button.property_expression("value"),
                    self.period_combo_box.property_expression("active"),
                ],
                closure!(|obj: Self::Type,
                          year: i32,
                          month: i32,
                          day: i32,
                          hour: f64,
                          minute: f64,
                          period: i32| {
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
                .chain_closure::<String>(closure!(|_: glib::Object, unix: i64| {
                    glib::DateTime::from_unix_local(unix)
                        .unwrap()
                        .format(
                            // Translators: This is a date time format (https://valadoc.org/glib-2.0/GLib.DateTime.format.html)
                            &gettext("%x %H:%M %p"),
                        )
                        .unwrap_or_else(|_| gettext("Invalid date format").into())
                }))
                .bind(&*self.prune_until_label, "label", Some(obj));

            self.pods_settings
                .bind("prune-all-images", &*self.prune_all_switch, "active")
                .build();

            self.pods_settings
                .bind(
                    "prune-external-images",
                    &*self.prune_external_switch,
                    "active",
                )
                .build();

            let (hour, minute) = glib::DateTime::now_local()
                .map(|now| (now.hour(), now.minute()))
                .unwrap_or((0, 0));

            self.hour_spin_button.set_value(hour as f64);
            self.minute_spin_button.set_value(minute as f64);
            self.period_combo_box
                .set_active(Some(if hour < 12 { 0 } else { 1 }));
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.header_bar.unparent();
            self.preferences_page.unparent();
        }
    }

    impl WidgetImpl for ImagesPrunePage {}
}

glib::wrapper! {
    pub(crate) struct ImagesPrunePage(ObjectSubclass<imp::ImagesPrunePage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<Option<&model::Client>> for ImagesPrunePage {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::new(&[("client", &client)]).expect("Failed to create ImagesPrunePage")
    }
}

impl ImagesPrunePage {
    pub(crate) fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    pub(crate) fn has_prune_until_filter(&self) -> bool {
        self.imp().prune_until_expander_row.enables_expansion()
    }

    pub(crate) fn prune_until_timestamp(&self) -> i64 {
        self.imp().prune_until_timestamp.get()
    }

    fn set_prune_until_timestamp(&self, value: i64) {
        if self.prune_until_timestamp() == value {
            return;
        }
        self.imp().prune_until_timestamp.set(value);
        self.notify("prune-until-timestamp");
    }

    fn load_time_format(&self) {
        let imp = self.imp();

        match imp.desktop_settings.get::<String>("clock-format").as_str() {
            "12h" => {
                imp.hour_adjustment.set_upper(11.0);
                imp.period_combo_box.set_visible(true);
                imp.time_format.set(TimeFormat::Hours12);
            }
            other => {
                if other != "24h" {
                    log::warn!("Unknown time format '{other}'. Falling back to '24h'.");
                }
                imp.hour_adjustment.set_upper(23.0);
                imp.period_combo_box.set_visible(false);
                imp.time_format.set(TimeFormat::Hours24);
            }
        }
    }

    fn prune(&self) {
        let imp = self.imp();
        self.client().unwrap().prune(
            podman::opts::ImagePruneOpts::builder()
                .all(imp.pods_settings.get("prune-all-images"))
                .external(imp.pods_settings.get("prune-external-images"))
                .filter(if self.has_prune_until_filter() {
                    Some(podman::opts::ImagePruneFilter::Until(
                        self.prune_until_timestamp().to_string(),
                    ))
                } else {
                    None
                })
                .build(),
            clone!(@weak self as obj => move |result| {
                match result {
                    Ok(_) => utils::show_toast(
                        &obj,
                        &gettext("All images have been pruned"),
                    ),
                    Err(e) => {
                        log::error!("Error on pruning images: {e}");
                        utils::show_error_toast(
                            &obj,
                            &gettext("Error on pruning images"),
                            &e.to_string()
                        );
                    }
                }
            }),
        )
    }
}

fn setup_time_spin_button(spin_button: &gtk::SpinButton) {
    spin_button.set_text(&format!("{:02}", spin_button.value()));
    spin_button.connect_output(|spin_button| {
        spin_button.set_text(&format!("{:02}", spin_button.value()));
        gtk::Inhibit(true)
    });
}
