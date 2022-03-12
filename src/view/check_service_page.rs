use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/check-service-page.ui")]
    pub(crate) struct CheckServicePage {
        #[template_child]
        pub(super) header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub(super) status_page: TemplateChild<adw::StatusPage>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CheckServicePage {
        const NAME: &'static str = "CheckServicePage";
        type Type = super::CheckServicePage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CheckServicePage {
        fn dispose(&self, _obj: &Self::Type) {
            self.header_bar.unparent();
            self.status_page.unparent();
        }
    }

    impl WidgetImpl for CheckServicePage {}
}

glib::wrapper! {
    pub(crate) struct CheckServicePage(ObjectSubclass<imp::CheckServicePage>)
        @extends gtk::Widget;
}
