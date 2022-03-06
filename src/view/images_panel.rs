use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::utils::ToTypedListModel;
use crate::{model, view};

mod imp {
    use gettextrs::gettext;
    use gtk::glib::closure;
    use gtk::CompositeTemplate;
    use once_cell::sync::Lazy;
    use once_cell::unsync::OnceCell;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/images-panel.ui")]
    pub struct ImagesPanel {
        pub image_list: OnceCell<model::ImageList>,
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub progress_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub image_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagesPanel {
        const NAME: &'static str = "ImagesPanel";
        type Type = super::ImagesPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
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
                vec![glib::ParamSpecObject::new(
                    "image-list",
                    "Image List",
                    "The list of images",
                    model::ImageList::static_type(),
                    glib::ParamFlags::READABLE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image-list" => obj.image_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let image_list_expr = Self::Type::this_expression("image-list");

            let fetched_params = &[
                image_list_expr
                    .chain_property::<model::ImageList>("fetched")
                    .upcast(),
                image_list_expr
                    .chain_property::<model::ImageList>("to-fetch")
                    .upcast(),
            ];

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

            let image_len_expr = image_list_expr.chain_property::<model::ImageList>("len");

            image_len_expr
                .chain_closure::<bool>(closure!(|_: glib::Object, len: u32| { len == 0 }))
                .bind(&*self.spinner, "visible", Some(obj));

            image_len_expr
                .chain_closure::<bool>(closure!(|_: glib::Object, len: u32| { len > 0 }))
                .bind(&*self.scrolled_window, "visible", Some(obj));

            gtk::ClosureExpression::new::<f64, _, _>(
                fetched_params,
                closure!(|_: glib::Object, fetched: u32, to_fetch: u32| {
                    fetched as f64 / to_fetch as f64
                }),
            )
            .bind(&*self.image_group, "description", Some(obj));

            gtk::ClosureExpression::new::<Option<String>, _, _>(
                [
                    &fetched_params[0],
                    &fetched_params[1],
                    &image_list_expr
                        .chain_property::<model::ImageList>("len")
                        .upcast(),
                ],
                closure!(
                    |images: Self::Type, fetched: u32, to_fetch: u32, len: u32| {
                        if fetched == to_fetch {
                            let list = images.image_list();
                            Some(gettext!(
                                "{} images total, {}  {} unused images, {}",
                                len,
                                glib::format_size(list.total_size()),
                                list.num_unused_images(),
                                glib::format_size(list.unused_size()),
                            ))
                        } else {
                            None
                        }
                    }
                ),
            )
            .bind(&*self.image_group, "description", Some(obj));

            self.list_box.bind_model(Some(obj.image_list()), |item| {
                view::ImageRow::from(item.downcast_ref().unwrap()).upcast()
            })
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.header_bar.unparent();
            self.overlay.unparent();
        }
    }
    impl WidgetImpl for ImagesPanel {}
}

glib::wrapper! {
    pub struct ImagesPanel(ObjectSubclass<imp::ImagesPanel>)
        @extends gtk::Widget;
}

impl Default for ImagesPanel {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create ImagesPanel")
    }
}

impl ImagesPanel {
    pub fn image_list(&self) -> &model::ImageList {
        self.imp().image_list.get_or_init(model::ImageList::default)
    }

    fn show_prune_dialog(&self) {
        let dialog = view::ImagesPruneDialog::from(self.image_list());
        dialog.set_transient_for(Some(
            &self.root().unwrap().downcast::<gtk::Window>().unwrap(),
        ));
        dialog.run_async(|dialog, response| {
            if matches!(response, gtk::ResponseType::Accept) {
                dialog
                    .images_to_prune()
                    .unwrap()
                    .to_owned()
                    .to_typed_list_model::<model::Image>()
                    .iter()
                    .for_each(|image| {
                        image.delete(|_| {
                            // TODO: Show a toast notification
                        })
                    });
            }
            dialog.close();
        });
    }
}
