use std::cell::Cell;

use adw::subclass::prelude::*;
use adw::traits::ExpanderRowExt;
use gettextrs::gettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::api;
use crate::model;
use crate::utils;
use crate::view;
use crate::window::Window;

#[derive(Clone, Copy, Debug)]
enum TimeFormat {
    Hours12,
    Hours24,
}

impl Default for TimeFormat {
    fn default() -> Self {
        Self::Hours24
    }
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
        pub(super) image_list: WeakRef<model::ImageList>,
        // pub(super) images_to_prune: OnceCell<gtk::NoSelection>,
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
        // #[template_child]
        // pub(super) preview_preferences_group: TemplateChild<adw::PreferencesGroup>,
        // #[template_child]
        // pub(super) scrolled_window: TemplateChild<gtk::ScrolledWindow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagesPrunePage {
        const NAME: &'static str = "ImagesPrunePage";
        type Type = super::ImagesPrunePage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("navigation.go-first", None, move |widget, _, _| {
                widget.navigate_to_first();
            });
            klass.install_action("navigation.back", None, move |widget, _, _| {
                widget.navigate_back();
            });

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
                        "image-list",
                        "Image List",
                        "The list of images",
                        model::ImageList::static_type(),
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
                    // glib::ParamSpecObject::new(
                    //     "images-to-prune",
                    //     "Images To Prune",
                    //     "The images to prune",
                    //     gtk::NoSelection::static_type(),
                    //     glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    // ),
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
                "image-list" => self.image_list.set(value.get().unwrap()),
                "prune-until-timestamp" => obj.set_prune_until_timestamp(value.get().unwrap()),
                // "images-to-prune" => obj.set_images_to_prune(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image-list" => obj.image_list().to_value(),
                "prune-until-timestamp" => obj.prune_until_timestamp().to_value(),
                // "images-to-prune" => obj.images_to_prune().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let image_list = obj.image_list().unwrap();
            obj.action_set_enabled("images.prune", !image_list.pruning());
            image_list.connect_notify_local(
                Some("pruning"),
                clone!(@weak obj => move |list, _| {
                    obj.action_set_enabled("images.prune", !list.pruning());
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

            // let image_list = self.image_list.get().unwrap();

            // let filter =
            //     gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
            //         let imp = obj.imp();

            //         let image = item.downcast_ref::<model::Image>().unwrap();
            //         image.dangling()
            //             || (imp.prune_all_switch.is_active() && image.containers() == 0)
            //                 && (!obj.has_prune_until_filter()
            //                     || image.created() < imp.prune_until_timestamp.get())
            //     }));
            // self.prune_all_switch.connect_notify_local(
            //     Some("active"),
            //     clone!(@weak filter => move |_, _| {
            //         filter.changed(gtk::FilterChange::Different)
            //     }),
            // );
            // self.prune_external_switch.connect_notify_local(
            //     Some("active"),
            //     clone!(@weak filter => move |_, _| {
            //         filter.changed(gtk::FilterChange::Different)
            //     }),
            // );
            // obj.connect_notify_local(
            //     Some("prune-until-timestamp"),
            //     clone!(@weak filter => move |_, _| {
            //         filter.changed(gtk::FilterChange::Different)
            //     }),
            // );
            // image_list.connect_notify_local(
            //     Some("fetched"),
            //     clone!(@weak filter => move |_ ,_| filter.changed(gtk::FilterChange::Different)),
            // );

            // obj.set_images_to_prune(gtk::NoSelection::new(Some(&gtk::FilterListModel::new(
            //     Some(&gtk::SortListModel::new(
            //         Some(image_list),
            //         Some(&gtk::CustomSorter::new(|obj1, obj2| {
            //             let image1 = obj1.downcast_ref::<model::Image>().unwrap();
            //             let image2 = obj2.downcast_ref::<model::Image>().unwrap();

            //             if image1.repo_tags().is_empty() {
            //                 if image2.repo_tags().is_empty() {
            //                     image1.id().cmp(image2.id()).into()
            //                 } else {
            //                     gtk::Ordering::Larger
            //                 }
            //             } else if image2.repo_tags().is_empty() {
            //                 gtk::Ordering::Smaller
            //             } else {
            //                 image1.repo_tags().cmp(image2.repo_tags()).into()
            //             }
            //         })),
            //     )),
            //     Some(&filter),
            // ))));

            // obj.on_images_to_prune_changed();
            // obj.images_to_prune().unwrap().connect_items_changed(
            //     clone!(@weak obj => move |_, _, _, _| obj.on_images_to_prune_changed()),
            // );

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

    impl WidgetImpl for ImagesPrunePage {
        fn realize(&self, widget: &Self::Type) {
            self.parent_realize(widget);

            widget.action_set_enabled(
                "navigation.go-first",
                widget.previous_leaflet_overlay() != widget.root_leaflet_overlay(),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImagesPrunePage(ObjectSubclass<imp::ImagesPrunePage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::ImageList> for ImagesPrunePage {
    fn from(image_list: &model::ImageList) -> Self {
        glib::Object::new(&[("image-list", image_list)]).expect("Failed to create ImagesPrunePage")
    }
}

impl ImagesPrunePage {
    pub(crate) fn image_list(&self) -> Option<model::ImageList> {
        self.imp().image_list.upgrade()
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

    // pub(crate) fn images_to_prune(&self) -> Option<&gtk::NoSelection> {
    //     self.imp().images_to_prune.get()
    // }

    // pub(crate) fn set_images_to_prune(&self, value: gtk::NoSelection) {
    //     if self.images_to_prune() == Some(&value) {
    //         return;
    //     }
    //     self.imp().images_to_prune.set(value).unwrap();
    //     self.notify("images-to-prune");
    // }

    // fn on_images_to_prune_changed(&self) {
    //     let imp = self.imp();

    //     let num_images = self.images_to_prune().unwrap().n_items();
    //     let has_images = num_images > 0;

    //     imp.preview_preferences_group
    //         .set_description(Some(&if has_images {
    //             gettext!("{} images can be pruned.", num_images)
    //         } else {
    //             gettext("No images to be pruned.")
    //         }));

    //     imp.button_prune.set_sensitive(has_images);
    //     imp.scrolled_window.set_visible(has_images);
    // }

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

    fn navigate_to_first(&self) {
        self.root_leaflet_overlay().hide_details();
    }

    fn navigate_back(&self) {
        self.previous_leaflet_overlay().hide_details();
    }

    fn previous_leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::find_leaflet_overlay(self)
    }

    fn root_leaflet_overlay(&self) -> view::LeafletOverlay {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .leaflet_overlay()
    }

    fn prune(&self) {
        let imp = self.imp();
        self.image_list().unwrap().prune(
            api::ImagePruneOpts::builder()
                .all(imp.pods_settings.get("prune-all-images"))
                .external(imp.pods_settings.get("prune-external-images"))
                .filter(if self.has_prune_until_filter() {
                    Some(api::ImagePruneFilter::Until(
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

// {
//     let imp = obj.imp();
//     image_list.prune(
//         api::ImagePruneOpts::builder()
//             .all(imp.settings.get("prune-all-images"))
//             .external(imp.settings.get("prune-external-images"))
//             .filter(if dialog.has_prune_until_filter() {
//                 Some(api::ImagePruneFilter::Until(
//                     dialog.prune_until_timestamp().to_string())
//                 )
//             } else {
//                 None
//             })
//             .build(),
//         clone!(@weak obj => move |result| {
//             obj.show_toast(&match result {
//                 Ok(reports) => match reports.as_ref().and_then(|reports| {
//                     if reports.is_empty() {
//                         None
//                     } else {
//                         Some(reports)
//                     }
//                 }) {
//                     Some(reports) => {
//                         let (num_images, num_errors, total_size) = reports
//                             .iter()
//                             .map(|report| match report.err {
//                                 Some(_) => (0, 1, 0),
//                                 None => (1, 0, report.size.unwrap_or(0)),
//                             })
//                             .fold((0, 0, 0), |acc, i| {
//                                 (acc.0 + i.0, acc.1 + i.1, acc.2 + i.2)
//                             });
//                         gettext!(
//                             // Translators: The first two placeholders are for a numbers, the last for disk space.
//                             "{} images pruned ({} errors). {} space reclaimed.",
//                             num_images,
//                             if num_errors == 0 {
//                                 Cow::Borrowed("no")
//                             } else {
//                                 Cow::Owned(num_errors.to_string())
//                             },
//                             glib::format_size(total_size as u64),
//                         )
//                     }

//                     None => gettext("There are no images to be pruned."),
//                 },
//                 Err(e) => gettext!(
//                     // Translators: "{}" is a placeholder for an error message.
//                     "Error on pruning images: '{}'",
//                     e
//                 ),
//             });
//             op();
//         }),
//     )
// }
