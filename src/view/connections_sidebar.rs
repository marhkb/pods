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
    #[properties(wrapper_type = super::ConnectionsSidebar)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/connections_sidebar.ui")]
    pub(crate) struct ConnectionsSidebar {
        #[property(get, set, nullable)]
        pub(super) connection_manager: glib::WeakRef<model::ConnectionManager>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConnectionsSidebar {
        const NAME: &'static str = "PdsConnectionsSidebar";
        type Type = super::ConnectionsSidebar;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.set_css_name("connectionssidebar");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ConnectionsSidebar {
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

    impl WidgetImpl for ConnectionsSidebar {}
}

glib::wrapper! {
    pub(crate) struct ConnectionsSidebar(ObjectSubclass<imp::ConnectionsSidebar>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
