use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

mod imp {
    use gtk::CompositeTemplate;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/start-service-page.ui")]
    pub(crate) struct StartServicePage {
        #[template_child]
        pub(super) header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub(super) status_page: TemplateChild<adw::StatusPage>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StartServicePage {
        const NAME: &'static str = "StartServicePage";
        type Type = super::StartServicePage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for StartServicePage {
        fn dispose(&self, _obj: &Self::Type) {
            self.header_bar.unparent();
            self.status_page.unparent();
        }
    }

    impl WidgetImpl for StartServicePage {}
}

glib::wrapper! {
    pub(crate) struct StartServicePage(ObjectSubclass<imp::StartServicePage>)
        @extends gtk::Widget;
}
