use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::NetworksRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/networks_row.ui")]
    pub(crate) struct NetworksRow {
        #[property(get, set)]
        pub(super) volume_list: glib::WeakRef<model::VolumeList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NetworksRow {
        const NAME: &'static str = "PdsNetworksRow";
        type Type = super::NetworksRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NetworksRow {
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

    impl WidgetImpl for NetworksRow {}
}

glib::wrapper! {
    pub(crate) struct NetworksRow(ObjectSubclass<imp::NetworksRow>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::Actionable, gtk::ConstraintTarget;
}
