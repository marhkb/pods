use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};

use crate::model;

mod imp {
    use adw::subclass::prelude::*;
    use gtk::glib::clone;
    use once_cell::sync::Lazy;
    use once_cell::unsync::OnceCell;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/images-prune-dialog.ui")]
    pub(crate) struct ImagesPruneDialog {
        pub(super) images_to_prune: OnceCell<gtk::NoSelection>,
        #[template_child]
        pub(super) button_prune: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) list_view: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagesPruneDialog {
        const NAME: &'static str = "ImagesPruneDialog";
        type Type = super::ImagesPruneDialog;
        type ParentType = gtk::Dialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagesPruneDialog {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "images-to-prune",
                    "Images To Prune",
                    "The images to prune",
                    gtk::NoSelection::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "images-to-prune" => self.images_to_prune.set(value.get().unwrap()).unwrap(),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "images-to-prune" => obj.images_to_prune().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.set_visible();
            obj.images_to_prune()
                .unwrap()
                .connect_items_changed(clone!(@weak obj => move |_, _, _, _| obj.set_visible()));
        }
    }

    impl WidgetImpl for ImagesPruneDialog {}
    impl WindowImpl for ImagesPruneDialog {}
    impl DialogImpl for ImagesPruneDialog {}
}

glib::wrapper! {
    pub(crate) struct ImagesPruneDialog(ObjectSubclass<imp::ImagesPruneDialog>)
        @extends gtk::Widget, gtk::Window, gtk::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl From<&model::ImageList> for ImagesPruneDialog {
    fn from(images: &model::ImageList) -> Self {
        glib::Object::new(&[
            (
                "images-to-prune",
                &gtk::NoSelection::new(Some(&gtk::FilterListModel::new(
                    Some(images),
                    Some(&gtk::CustomFilter::new(|obj| {
                        let image = obj.downcast_ref::<model::Image>().unwrap();
                        image.dangling() || image.containers() == 0
                    })),
                ))),
            ),
            ("use-header-bar", &1),
        ])
        .expect("Failed to create ImagesPruneDialog")
    }
}

impl ImagesPruneDialog {
    pub(crate) fn images_to_prune(&self) -> Option<&gtk::NoSelection> {
        self.imp().images_to_prune.get()
    }

    fn set_visible(&self) {
        let has_images = self.images_to_prune().unwrap().n_items() > 0;

        let imp = self.imp();
        imp.button_prune.set_sensitive(has_images);
        imp.status_page.set_visible(!has_images);
        imp.list_view.set_visible(has_images);
    }
}
