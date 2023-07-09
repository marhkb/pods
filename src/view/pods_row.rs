use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PodsRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/pods_row.ui")]
    pub(crate) struct PodsRow {
        #[property(get, set)]
        pub(super) pod_list: glib::WeakRef<model::PodList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodsRow {
        const NAME: &'static str = "PdsPodsRow";
        type Type = super::PodsRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodsRow {
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

    impl WidgetImpl for PodsRow {}
}

glib::wrapper! {
    pub(crate) struct PodsRow(ObjectSubclass<imp::PodsRow>) @extends gtk::Widget;
}
