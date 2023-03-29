use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

const ACTION_CREATE_CONTAINER: &str = "image-menu-button.create-container";
const ACTION_DELETE_IMAGE: &str = "image-menu-button.delete-image";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::MenuButton)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image/menu-button.ui")]
    pub(crate) struct MenuButton {
        #[property(get, set, nullable)]
        pub(super) image: glib::WeakRef<model::Image>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MenuButton {
        const NAME: &'static str = "PdsImageMenuButton";
        type Type = super::MenuButton;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_CREATE_CONTAINER, None, |widget, _, _| {
                widget.create_container();
            });
            klass.install_action(ACTION_DELETE_IMAGE, None, |widget, _, _| {
                widget.delete_image();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MenuButton {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            Self::Type::this_expression("css-classes").bind(
                &*self.menu_button,
                "css-classes",
                Some(obj),
            );

            let to_be_deleted_expr = Self::Type::this_expression("image")
                .chain_property::<model::Image>("to-be-deleted");

            to_be_deleted_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, to_be_deleted: bool| {
                    !to_be_deleted
                }))
                .bind(&*self.menu_button, "sensitive", Some(obj));

            to_be_deleted_expr.watch(
                Some(obj),
                clone!(@weak obj => move || {
                    obj.action_set_enabled(
                        ACTION_DELETE_IMAGE,
                        obj.image().map(|image| !image.to_be_deleted()).unwrap_or(false)
                    );
                }),
            );
        }

        fn dispose(&self) {
            utils::ChildIter::from(self.obj().upcast_ref()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for MenuButton {}
}

glib::wrapper! {
    pub(crate) struct MenuButton(ObjectSubclass<imp::MenuButton>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl MenuButton {
    pub(crate) fn delete_image(&self) {
        super::delete_image_show_confirmation(self.upcast_ref(), self.image());
    }

    pub(crate) fn create_container(&self) {
        super::create_container(self.upcast_ref(), self.image());
    }
}
