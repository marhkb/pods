use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;

const ACTION_DELETE_IMAGE: &str = "image-menu-button.delete-image";
const ACTION_INSPECT_IMAGE: &str = "image-menu-button.inspect-image";
const ACTION_PULL_LATEST: &str = "image-menu-button.pull-latest";
const ACTION_CREATE_CONTAINER: &str = "image-menu-button.create-container";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image/menu-button.ui")]
    pub(crate) struct MenuButton {
        pub(super) image: WeakRef<model::Image>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MenuButton {
        const NAME: &'static str = "PdsImageMenuButton";
        type Type = super::MenuButton;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_DELETE_IMAGE, None, move |widget, _, _| {
                widget.delete_image();
            });
            klass.install_action(ACTION_INSPECT_IMAGE, None, move |widget, _, _| {
                widget.show_inspection();
            });
            klass.install_action(ACTION_PULL_LATEST, None, move |widget, _, _| {
                widget.pull_latest();
            });
            klass.install_action(ACTION_CREATE_CONTAINER, None, move |widget, _, _| {
                widget.create_container();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MenuButton {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "image",
                        "Image",
                        "The image of this image menu button",
                        model::Image::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecBoolean::new(
                        "primary",
                        "Primary",
                        "Whether the image menu button acts as a primary menu",
                        false,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "image" => obj.set_image(value.get().unwrap_or_default()),
                "primary" => obj.set_primary(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image" => obj.image().to_value(),
                "primary" => obj.is_primary().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            self.menu_button
                .connect_primary_notify(clone!(@weak obj => move |_| {
                    obj.notify("primary")
                }));

            Self::Type::this_expression("css-classes").bind(
                &*self.menu_button,
                "css-classes",
                Some(obj),
            );

            let to_be_deleted_expr = Self::Type::this_expression("image")
                .chain_property::<model::Image>("to-be-deleted");

            to_be_deleted_expr
                .chain_closure::<String>(closure!(|_: glib::Object, to_be_deleted: bool| {
                    if to_be_deleted {
                        "ongoing"
                    } else {
                        "menu"
                    }
                }))
                .bind(&*self.stack, "visible-child-name", Some(obj));

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

        fn dispose(&self, obj: &Self::Type) {
            utils::ChildIter::from(obj).for_each(|child| child.unparent());
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
    pub(crate) fn image(&self) -> Option<model::Image> {
        self.imp().image.upgrade()
    }

    pub(crate) fn set_image(&self, value: Option<&model::Image>) {
        if self.image().as_ref() == value {
            return;
        }

        let imp = self.imp();

        imp.image.set(value);
        self.notify("image");
    }

    pub(crate) fn is_primary(&self) -> bool {
        self.imp().menu_button.is_primary()
    }

    pub(crate) fn set_primary(&self, value: bool) {
        self.imp().menu_button.set_primary(value);
    }

    fn delete_image(&self) {
        super::delete_image_show_confirmation(self.upcast_ref(), self.image());
    }

    fn show_inspection(&self) {
        super::show_inspection(&utils::find_leaflet_overlay(self), self.image());
    }

    fn pull_latest(&self) {
        super::pull_latest(None, self.image());
    }

    fn create_container(&self) {
        super::create_container(&utils::find_leaflet_overlay(self), self.image());
    }
}
