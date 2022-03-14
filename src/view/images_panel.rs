use std::cell::Cell;

use gettextrs::gettext;
use gtk::glib::{clone, closure};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, CompositeTemplate};
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::utils::ToTypedListModel;
use crate::window::Window;
use crate::{config, model, view};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/images-panel.ui")]
    pub(crate) struct ImagesPanel {
        pub(super) image_list: OnceCell<model::ImageList>,
        pub(super) filter: OnceCell<gtk::CustomFilter>,
        pub(super) show_intermediates: Cell<bool>,
        #[template_child]
        pub(super) status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) progress_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) preferences_page: TemplateChild<adw::PreferencesPage>,
        #[template_child]
        pub(super) image_group: TemplateChild<adw::PreferencesGroup>,
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
            klass.install_property_action("images.show-intermediates", "show-intermediates");
            klass.install_action("images.prune-unused", None, move |widget, _, _| {
                widget.show_prune_dialog();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagesPanel {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "image-list",
                        "Image List",
                        "The list of images",
                        model::ImageList::static_type(),
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecBoolean::new(
                        "show-intermediates",
                        "Show Intermediates",
                        "Whether to also show intermediate images",
                        false,
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
                "show-intermediates" => obj.set_show_intermediates(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image-list" => obj.image_list().to_value(),
                "show-intermediates" => obj.show_intermediates().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let image_list_expr = Self::Type::this_expression("image-list");
            let image_len_expr = image_list_expr.chain_property::<model::ImageList>("len");
            let fetched_params = &[
                image_list_expr
                    .chain_property::<model::ImageList>("fetched")
                    .upcast(),
                image_list_expr
                    .chain_property::<model::ImageList>("to-fetch")
                    .upcast(),
            ];

            gtk::ClosureExpression::new::<bool, _, _>(
                &[
                    image_len_expr.clone(),
                    image_list_expr.chain_property::<model::ImageList>("listing"),
                ],
                closure!(|_: glib::Object, len: u32, listing: bool| len == 0 && listing),
            )
            .bind(&*self.status_page, "visible", Some(obj));

            image_len_expr
                .chain_closure::<bool>(closure!(|_: glib::Object, len: u32| { len > 0 }))
                .bind(&*self.overlay, "visible", Some(obj));

            gtk::ClosureExpression::new::<f64, _, _>(
                fetched_params,
                closure!(|_: glib::Object, fetched: u32, to_fetch: u32| {
                    fetched as f64 / to_fetch as f64
                }),
            )
            .bind(&*self.progress_bar, "fraction", Some(obj));

            gtk::ClosureExpression::new::<String, _, _>(
                fetched_params,
                closure!(|_: glib::Object, fetched: u32, to_fetch: u32| {
                    if fetched == to_fetch {
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

            gtk::ClosureExpression::new::<Option<String>, _, _>(
                [
                    &fetched_params[0],
                    &fetched_params[1],
                    &image_list_expr
                        .chain_property::<model::ImageList>("len")
                        .upcast(),
                ],
                closure!(|panel: Self::Type, fetched: u32, to_fetch: u32, len: u32| {
                    if fetched == to_fetch {
                        let list = panel.image_list();
                        Some(
                            // Translators: There's a wide space (U+2002) between the two {} {}.
                            gettext!(
                                "{} images total, {} {} unused images, {}",
                                len,
                                glib::format_size(list.total_size()),
                                list.num_unused_images(),
                                glib::format_size(list.unused_size()),
                            ),
                        )
                    } else {
                        None
                    }
                }),
            )
            .bind(&*self.image_group, "description", Some(obj));

            let filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    if obj.show_intermediates() {
                        let image = item
                            .downcast_ref::<model::Image>()
                            .unwrap();
                        !image.dangling() && image.containers() > 0
                    } else {
                        true
                    }
                }));
            let filter_model = gtk::FilterListModel::new(Some(obj.image_list()), Some(&filter));

            self.list_box.bind_model(Some(&filter_model), |item| {
                view::ImageRow::from(item.downcast_ref().unwrap()).upcast()
            });

            self.filter.set(filter).unwrap();

            gio::Settings::new(config::APP_ID)
                .bind("show-intermediate-images", obj, "show-intermediates")
                .build();
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.status_page.unparent();
            self.overlay.unparent();
        }
    }
    impl WidgetImpl for ImagesPanel {}
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
    pub(crate) fn image_list(&self) -> &model::ImageList {
        self.imp().image_list.get_or_init(model::ImageList::default)
    }

    pub(crate) fn show_intermediates(&self) -> bool {
        self.imp().show_intermediates.get()
    }

    pub(crate) fn set_show_intermediates(&self, value: bool) {
        if self.show_intermediates() == value {
            return;
        }
        let imp = self.imp();
        imp.show_intermediates.set(value);
        imp.filter
            .get()
            .unwrap()
            .changed(gtk::FilterChange::Different);

        self.notify("show-intermediates");
    }

    pub(crate) fn show_prune_dialog(&self) {
        let dialog = view::ImagesPruneDialog::from(self.image_list());
        dialog.set_transient_for(Some(
            &self.root().unwrap().downcast::<gtk::Window>().unwrap(),
        ));
        dialog.run_async(clone!(@weak self as obj => move |dialog, response| {
            if matches!(response, gtk::ResponseType::Accept) {
                dialog
                    .images_to_prune()
                    .unwrap()
                    .to_owned()
                    .to_typed_list_model::<model::Image>()
                    .iter()
                    .for_each(|image| {
                        let id = image.id().to_owned();
                        image.delete(clone!(@weak obj => move |_| {
                            obj.root().unwrap().downcast::<Window>().unwrap().show_toast(
                                &adw::Toast::builder()
                                    .title(&gettext!("Error on pruning image '{}'", id))
                                    .timeout(3)
                                    .priority(adw::ToastPriority::High)
                                    .build()
                            );
                        }))
                    });
            }
            dialog.close();
        }));
    }
}
