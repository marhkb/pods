use adw::traits::BinExt;
use gettextrs::gettext;
use gettextrs::ngettext;
use gtk::gdk;
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
        pub(super) sorter: OnceCell<gtk::Sorter>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) images_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) show_intermediates_switch: TemplateChild<gtk::Switch>,
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

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                "images.pull",
                None,
            );
            klass.install_action("images.pull", None, move |widget, _, _| {
                widget.show_download_page();
            });

            klass.install_action("images.prune-unused", None, move |widget, _, _| {
                widget.show_prune_page();
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
            self.settings
                .bind(
                    "show-intermediate-images",
                    &*self.show_intermediates_switch,
                    "active",
                )
                .build();

            let image_list_expr = Self::Type::this_expression("image-list");
            let image_list_len_expr = image_list_expr.chain_property::<model::ImageList>("len");

            gtk::ClosureExpression::new::<String, _, _>(
                &[
                    image_list_len_expr.clone(),
                    image_list_expr.chain_property::<model::ImageList>("listing"),
                ],
                closure!(|_: Self::Type, len: u32, listing: bool| {
                    if len == 0 && listing {
                        "spinner"
                    } else {
                        "images"
                    }
                }),
            )
            .bind(&*self.main_stack, "visible-child-name", Some(obj));

            gtk::ClosureExpression::new::<String, _, _>(
                &[image_list_expr, image_list_len_expr],
                closure!(|_: glib::Object, list: Option<model::ImageList>, _: u32| {
                    match list {
                        Some(list) => {
                            let len = list.n_items();

                            if len == 0 {
                                gettext("No images found")
                            } else if len == 1 {
                                if list.num_unused_images() == 0 {
                                    gettext("1 image, used")
                                } else {
                                    gettext("1 image, unused")
                                }
                            } else {
                                ngettext!(
                                    // Translators: There's a wide space (U+2002) between ", {}".
                                    "{} image total, {} {} unused, {}",
                                    "{} images total, {} {} unused, {}",
                                    len,
                                    len,
                                    glib::format_size(list.total_size()),
                                    list.num_unused_images(),
                                    glib::format_size(list.unused_size()),
                                )
                            }
                        }
                        None => gettext("No images found"),
                    }
                }),
            )
            .bind(&*self.images_group, "description", Some(obj));

            let properties_filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    obj.imp().show_intermediates_switch.is_active()
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
            self.sorter.set(sorter.upcast()).unwrap();
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.main_stack.unparent();
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
    pub(crate) fn image_list(&self) -> Option<model::ImageList> {
        self.imp().image_list.upgrade()
    }

    pub(crate) fn set_image_list(&self, value: &model::ImageList) {
        if self.image_list().as_ref() == Some(value) {
            return;
        }

        // TODO: For multi-client: Figure out whether signal handlers need to be disconnected.
        let imp = self.imp();

        let model = gtk::SortListModel::new(
            Some(&gtk::FilterListModel::new(
                Some(value),
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

    fn show_download_page(&self) {
        let leaflet_overlay = utils::find_leaflet_overlay(self);

        if leaflet_overlay.child().is_none() {
            leaflet_overlay.show_details(&view::ImagePullPage::from(self.client().as_ref()));
        }
    }

    fn show_prune_page(&self) {
        let leaflet_overlay = utils::find_leaflet_overlay(self);

        if leaflet_overlay.child().is_none() {
            leaflet_overlay.show_details(&view::ImagesPrunePage::from(self.client().as_ref()));
        }
    }

    fn client(&self) -> Option<model::Client> {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .connection_manager()
            .client()
    }
}
