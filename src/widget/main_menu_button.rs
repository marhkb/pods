use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/main_menu_button.ui")]
    pub(crate) struct MainMenuButton;

    #[glib::object_subclass]
    impl ObjectSubclass for MainMenuButton {
        const NAME: &'static str = "PdsMainMenuButton";
        type Type = super::MainMenuButton;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MainMenuButton {
        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for MainMenuButton {}
}

glib::wrapper! {
    pub(crate) struct MainMenuButton(ObjectSubclass<imp::MainMenuButton>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
