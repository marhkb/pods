use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

mod imp {
    use gtk::CompositeTemplate;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/connection-lost-page.ui")]
    pub struct ConnectionLostPage {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConnectionLostPage {
        const NAME: &'static str = "ConnectionLostPage";
        type Type = super::ConnectionLostPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ConnectionLostPage {
        fn dispose(&self, _obj: &Self::Type) {
            self.header_bar.unparent();
            self.status_page.unparent();
        }
    }

    impl WidgetImpl for ConnectionLostPage {}
}

glib::wrapper! {
    pub struct ConnectionLostPage(ObjectSubclass<imp::ConnectionLostPage>)
        @extends gtk::Widget;
}
