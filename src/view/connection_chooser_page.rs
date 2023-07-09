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
    #[properties(wrapper_type = super::ConnectionChooserPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/connection_chooser_page.ui")]
    pub(crate) struct ConnectionChooserPage {
        #[property(get, set, nullable)]
        pub(crate) connection_manager: glib::WeakRef<model::ConnectionManager>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConnectionChooserPage {
        const NAME: &'static str = "PdsConnectionChooserPage";
        type Type = super::ConnectionChooserPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.set_css_name("connectionchooserpage");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ConnectionChooserPage {
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

    impl WidgetImpl for ConnectionChooserPage {}
}

glib::wrapper! {
    pub(crate) struct ConnectionChooserPage(ObjectSubclass<imp::ConnectionChooserPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
