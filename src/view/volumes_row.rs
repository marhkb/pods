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
    #[properties(wrapper_type = super::VolumesRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/volumes_row.ui")]
    pub(crate) struct VolumesRow {
        #[property(get, set)]
        pub(super) volume_list: glib::WeakRef<model::VolumeList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumesRow {
        const NAME: &'static str = "PdsVolumesRow";
        type Type = super::VolumesRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VolumesRow {
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
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for VolumesRow {}
}

glib::wrapper! {
    pub(crate) struct VolumesRow(ObjectSubclass<imp::VolumesRow>) @extends gtk::Widget;
}
