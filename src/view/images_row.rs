use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImagesRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/images_row.ui")]
    pub(crate) struct ImagesRow {
        #[property(get, set)]
        pub(super) image_list: glib::WeakRef<model::ImageList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagesRow {
        const NAME: &'static str = "PdsImagesRow";
        type Type = super::ImagesRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagesRow {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ImagesRow {}
}

glib::wrapper! {
    pub(crate) struct ImagesRow(ObjectSubclass<imp::ImagesRow>) @extends gtk::Widget;
}
