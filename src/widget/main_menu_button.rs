use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/main_menu_button.ui")]
    pub(crate) struct MainMenuButton {
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
    }

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
        fn constructed(&self) {
            self.parent_constructed();

            let popover_menu = self
                .menu_button
                .popover()
                .unwrap()
                .downcast::<gtk::PopoverMenu>()
                .unwrap();

            popover_menu.add_child(
                &panel::ThemeSelector::builder()
                    .action_name("win.theme")
                    .build(),
                "theme",
            );
        }

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
