use std::cell::Cell;

use adw::subclass::prelude::*;
use adw::traits::ExpanderRowExt;
use gettextrs::gettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;

const ACTION_PRUNE: &str = "images-prune-page.prune";

#[derive(Clone, Copy, Debug, Default)]
enum TimeFormat {
    Hours12,
    #[default]
    Hours24,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/images/prune-page.ui")]
    pub(crate) struct PrunePage {
        pub(super) desktop_settings: utils::DesktopSettings,
        pub(super) pods_settings: utils::PodsSettings,
        pub(super) time_format: Cell<TimeFormat>,
        pub(super) prune_until_timestamp: Cell<i64>,
        pub(super) client: glib::WeakRef<model::Client>,
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
        pub(super) period_drop_down: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) button_prune: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PrunePage {
        const NAME: &'static str = "PdsImagesPrunePage";
        type Type = super::PrunePage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_PRUNE, None, |widget, _, _| {
                widget.prune();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PrunePage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Client>("client")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecInt64::builder("prune-until-timestamp")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "client" => self.client.set(value.get().unwrap()),
                "prune-until-timestamp" => self
                    .instance()
                    .set_prune_until_timestamp(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.instance();
            match pspec.name() {
                "client" => obj.client().to_value(),
                "prune-until-timestamp" => obj.prune_until_timestamp().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            obj.load_time_format();
            self.desktop_settings.connect_changed(
                Some("clock-format"),
                clone!(@weak obj => move |_, _| {
                    obj.load_time_format();
                }),
            );

            setup_time_spin_button(&*self.hour_spin_button);
            setup_time_spin_button(&*self.minute_spin_button);

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
            self.period_drop_down
                .set_selected(if hour < 12 { 0 } else { 1 });
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.instance()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for PrunePage {}
}

glib::wrapper! {
    pub(crate) struct PrunePage(ObjectSubclass<imp::PrunePage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<Option<&model::Client>> for PrunePage {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::new::<Self>(&[("client", &client)])
    }
}

impl PrunePage {
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

    fn prune(&self) {
        let imp = self.imp();

        let action = self.client().unwrap().action_list().prune_images(
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
        );

        imp.leaflet_overlay
            .show_details(&view::ActionPage::from(&action));
    }
}

fn setup_time_spin_button(spin_button: &gtk::SpinButton) {
    spin_button.set_text(&format!("{:02}", spin_button.value()));
    spin_button.connect_output(|spin_button| {
        spin_button.set_text(&format!("{:02}", spin_button.value()));
        gtk::Inhibit(true)
    });
}
