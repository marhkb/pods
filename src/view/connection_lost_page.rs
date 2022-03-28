use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/connection-lost-page.ui")]
    pub(crate) struct ConnectionLostPage {
        #[template_child]
        pub(super) header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub(super) status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) button: TemplateChild<gtk::Button>,
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
    pub(crate) struct ConnectionLostPage(ObjectSubclass<imp::ConnectionLostPage>)
        @extends gtk::Widget;
}

impl ConnectionLostPage {
    pub(crate) fn set_enabled(&self, value: bool) {
        self.imp().button.set_sensitive(value);
    }
}
