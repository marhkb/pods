use std::borrow::Cow;

use gettextrs::gettext;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;

use crate::api;
use crate::model;
use crate::utils;
use crate::view;
use crate::window::Window;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/images-panel.ui")]
    pub(crate) struct ImagesPanel {
        pub(super) settings: utils::PodsSettings,
        pub(super) image_list: WeakRef<model::ImageList>,
        pub(super) properties_filter: OnceCell<gtk::Filter>,
        pub(super) search_filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) progress_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) images_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagesPanel {
        const NAME: &'static str = "ImagesPanel";
        type Type = super::ImagesPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagesPanel {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "image-list",
                    "Image List",
                    "The list of images",
                    model::ImageList::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
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
                "image-list" => obj.set_image_list(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image-list" => obj.image_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            self.settings.connect_changed(
                Some("show-intermediate-images"),
                clone!(@weak obj => move |_, _| obj.update_properties_filter()),
            );

            let image_list_expr = Self::Type::this_expression("image-list");
            let image_list_len_expr = image_list_expr.chain_property::<model::ImageList>("len");
            let fetched_params = &[
                image_list_expr
                    .chain_property::<model::ImageList>("fetched")
                    .upcast(),
                image_list_expr
                    .chain_property::<model::ImageList>("to-fetch")
                    .upcast(),
            ];

            gtk::ClosureExpression::new::<gtk::Widget, _, _>(
                &[
                    image_list_len_expr.clone(),
                    image_list_expr.chain_property::<model::ImageList>("listing"),
                ],
                closure!(|obj: Self::Type, len: u32, listing: bool| {
                    let imp = obj.imp();
                    if len == 0 && listing {
                        imp.spinner.upcast_ref::<gtk::Widget>().clone()
                    } else {
                        imp.overlay.upcast_ref::<gtk::Widget>().clone()
                    }
                }),
            )
            .bind(&*self.main_stack, "visible-child", Some(obj));

            gtk::ClosureExpression::new::<f64, _, _>(
                fetched_params,
                closure!(|_: glib::Object, fetched: u32, to_fetch: u32| {
                    f64::min(1.0, fetched as f64 / to_fetch as f64)
                }),
            )
            .bind(&*self.progress_bar, "fraction", Some(obj));

            gtk::ClosureExpression::new::<String, _, _>(
                fetched_params,
                closure!(|_: glib::Object, fetched: u32, to_fetch: u32| {
                    if fetched >= to_fetch {
                        "empty"
                    } else {
                        "bar"
                    }
                }),
            )
            .bind(&*self.progress_stack, "visible-child-name", Some(obj));

            gtk::Stack::this_expression("visible-child-name")
                .chain_closure::<u32>(closure!(|_: glib::Object, name: &str| {
                    match name {
                        "empty" => 0_u32,
                        "bar" => 1000,
                        _ => unreachable!(),
                    }
                }))
                .bind(
                    &*self.progress_stack,
                    "transition-duration",
                    Some(&*self.progress_stack),
                );

            gtk::ClosureExpression::new::<String, _, _>(
                &[image_list_expr, image_list_len_expr],
                closure!(|_: glib::Object, list: Option<model::ImageList>, _: u32| {
                    match list.filter(|list| list.len() > 0) {
                        Some(list) => gettext!(
                            // Translators: There's a wide space (U+2002) between the two {} {}.
                            "{} images total, {} {} unused images, {}",
                            list.len(),
                            glib::format_size(list.total_size()),
                            list.num_unused_images(),
                            glib::format_size(list.unused_size()),
                        ),
                        None => gettext("No images found"),
                    }
                }),
            )
            .bind(&*self.images_group, "description", Some(obj));

            let properties_filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    obj.show_intermediates()
                    || !item
                        .downcast_ref::<model::Image>()
                        .unwrap()
                        .repo_tags()
                        .is_empty()
                }));

            obj.connect_notify_local(
                Some("show-intermediates"),
                clone!(@weak obj => move |_ ,_| obj.update_properties_filter()),
            );

            let search_filter = gtk::CustomFilter::new(
                clone!(@weak obj => @default-return false, move |item| {
                    let image = item
                        .downcast_ref::<model::Image>()
                        .unwrap();
                    let query = obj.imp().search_entry.text();
                    let query = query.as_str();

                    image.id().contains(query) || image.repo_tags().iter().any(|s| s.contains(query))
                }),
            );

            self.search_entry
                .connect_search_changed(clone!(@weak obj => move |_| {
                    obj.update_search_filter();
                }));

            let sorter = gtk::CustomSorter::new(|obj1, obj2| {
                let image1 = obj1.downcast_ref::<model::Image>().unwrap();
                let image2 = obj2.downcast_ref::<model::Image>().unwrap();

                if image1.repo_tags().is_empty() {
                    if image2.repo_tags().is_empty() {
                        image1.id().cmp(image2.id()).into()
                    } else {
                        gtk::Ordering::Larger
                    }
                } else if image2.repo_tags().is_empty() {
                    gtk::Ordering::Smaller
                } else {
                    image1.repo_tags().cmp(image2.repo_tags()).into()
                }
            });

            self.properties_filter
                .set(properties_filter.upcast())
                .unwrap();
            self.search_filter.set(search_filter.upcast()).unwrap();
            self.sorter.set(sorter.upcast()).unwrap();
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.main_stack.unparent();
        }
    }

    impl WidgetImpl for ImagesPanel {
        fn root(&self, widget: &Self::Type) {
            self.parent_root(widget);
            self.search_bar
                .set_key_capture_widget(widget.root().as_ref());
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImagesPanel(ObjectSubclass<imp::ImagesPanel>)
        @extends gtk::Widget;
}

impl Default for ImagesPanel {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create ImagesPanel")
    }
}

impl ImagesPanel {
    pub(crate) fn image_list(&self) -> Option<model::ImageList> {
        self.imp().image_list.upgrade()
    }

    pub(crate) fn set_image_list(&self, value: &model::ImageList) {
        if self.image_list().as_ref() == Some(value) {
            return;
        }

        // TODO: For multi-client: Figure out whether signal handlers need to be disconnected.
        let imp = self.imp();

        value.connect_notify_local(
            Some("fetched"),
            clone!(@weak self as obj => move |_ ,_| {
                obj.update_properties_filter();
                obj.update_search_filter();
            }),
        );

        let model = gtk::SortListModel::new(
            Some(&gtk::FilterListModel::new(
                Some(&gtk::FilterListModel::new(
                    Some(value),
                    imp.search_filter.get(),
                )),
                imp.properties_filter.get(),
            )),
            imp.sorter.get(),
        );

        self.set_list_box_visibility(model.upcast_ref());
        model.connect_items_changed(clone!(@weak self as obj => move |model, _, _, _| {
            obj.set_list_box_visibility(model.upcast_ref());
        }));

        imp.list_box.bind_model(Some(&model), |item| {
            view::ImageRow::from(item.downcast_ref().unwrap()).upcast()
        });

        imp.image_list.set(Some(value));
        self.notify("image-list");
    }

    pub(crate) fn show_intermediates(&self) -> bool {
        self.imp().settings.get::<bool>("show-intermediate-images")
    }

    pub(crate) fn connect_search_button(&self, search_button: &gtk::ToggleButton) {
        search_button
            .bind_property("active", &*self.imp().search_bar, "search-mode-enabled")
            .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
            .build();
    }

    pub(crate) fn toggle_search(&self) {
        let imp = self.imp();
        if imp.search_bar.is_search_mode() {
            imp.search_bar.set_search_mode(false);
        } else {
            imp.search_bar.set_search_mode(true);
            imp.search_entry.grab_focus();
        }
    }

    pub(crate) fn show_prune_dialog<F>(&self, op: F)
    where
        F: FnOnce() + Clone + 'static,
    {
        let dialog = view::ImagesPruneDialog::default();
        dialog.set_transient_for(Some(
            &self.root().unwrap().downcast::<gtk::Window>().unwrap(),
        ));
        dialog.run_async(clone!(@weak self as obj => move |dialog, response| {
            match (response, obj.image_list()) {
                (gtk::ResponseType::Accept, Some(image_list)) => {
                    let imp = obj.imp();
                    image_list.prune(
                        api::ImagePruneOpts::builder()
                            .all(imp.settings.get("prune-all-images"))
                            .external(imp.settings.get("prune-external-images"))
                            .filter(if dialog.has_prune_until_filter() {
                                Some(api::ImagePruneFilter::Until(
                                    dialog.prune_until_timestamp().to_string())
                                )
                            } else {
                                None
                            })
                            .build(),
                        clone!(@weak obj => move |result| {
                            obj.show_toast(&match result {
                                Ok(reports) => match reports.as_ref().and_then(|reports| {
                                    if reports.is_empty() {
                                        None
                                    } else {
                                        Some(reports)
                                    }
                                }) {
                                    Some(reports) => {
                                        let (num_images, num_errors, total_size) = reports
                                            .iter()
                                            .map(|report| match report.err {
                                                Some(_) => (0, 1, 0),
                                                None => (1, 0, report.size.unwrap_or(0)),
                                            })
                                            .fold((0, 0, 0), |acc, i| {
                                                (acc.0 + i.0, acc.1 + i.1, acc.2 + i.2)
                                            });
                                        gettext!(
                                            // Translators: The first two placeholders are for a numbers, the last for disk space.
                                            "{} images pruned ({} errors). {} space reclaimed.",
                                            num_images,
                                            if num_errors == 0 {
                                                Cow::Borrowed("no")
                                            } else {
                                                Cow::Owned(num_errors.to_string())
                                            },
                                            glib::format_size(total_size as u64),
                                        )
                                    }

                                    None => gettext("There are no images to be pruned."),
                                },
                                Err(e) => gettext!(
                                    // Translators: "{}" is a placeholder for an error message.
                                    "Error on pruning images: '{}'",
                                    e
                                ),
                            });
                            op();
                        }),
                    )
                },
                _ => op(),
            }
            dialog.close();
        }));
    }

    fn show_toast(&self, title: &str) {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .show_toast(
                &adw::Toast::builder()
                    .title(title)
                    .timeout(3)
                    .priority(adw::ToastPriority::High)
                    .build(),
            );
    }

    fn set_list_box_visibility(&self, model: &gio::ListModel) {
        self.imp().list_box.set_visible(model.n_items() > 0);
    }

    pub(crate) fn update_properties_filter(&self) {
        self.imp()
            .properties_filter
            .get()
            .unwrap()
            .changed(gtk::FilterChange::Different);
    }

    pub(crate) fn update_search_filter(&self) {
        self.imp()
            .search_filter
            .get()
            .unwrap()
            .changed(gtk::FilterChange::Different);
    }
}
