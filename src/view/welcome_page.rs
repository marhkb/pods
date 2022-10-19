use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/welcome-page.ui")]
    pub(crate) struct WelcomePage;

    #[glib::object_subclass]
    impl ObjectSubclass for WelcomePage {
        const NAME: &'static str = "WelcomePage";
        type Type = super::WelcomePage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WelcomePage {
        fn dispose(&self) {
            utils::ChildIter::from(&*self.instance()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for WelcomePage {}
}

glib::wrapper! {
    pub(crate) struct WelcomePage(ObjectSubclass<imp::WelcomePage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
