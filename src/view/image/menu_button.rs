use adw::prelude::MessageDialogExtManual;
use adw::traits::MessageDialogExt;
use gettextrs::gettext;
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
use crate::view;

pub(crate) const ACTION_CREATE_CONTAINER: &str = "image-menu-button.create-container";
pub(crate) const ACTION_DELETE_IMAGE: &str = "image-menu-button.delete-image";

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

            klass.install_action(ACTION_CREATE_CONTAINER, None, move |widget, _, _| {
                widget.create_container();
            });
            klass.install_action(ACTION_DELETE_IMAGE, None, move |widget, _, _| {
                widget.delete();
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

    fn delete(&self) {
        if let Some(image) = self.image().as_ref() {
            let first_container = image.container_list().get(0);

            if image.containers() > 0 || first_container.is_some() {
                let dialog = adw::MessageDialog::builder()
                    .heading(&gettext("Confirm Forced Image Deletion"))
                    .body_use_markup(true)
                    .body(
                        &match first_container.as_ref().map(|c| c.name()) {
                            Some(id) => gettext!(
                                // Translators: The "{}" is a placeholder for the container name.
                                "Image is used by container <b>{}</b>. Deleting the image will also delete all its associated containers.",
                                id
                            ),
                            None => gettext(
                               "Image is used by a container. Deleting the image will also delete all its associated containers.",
                           ),
                        }

                    )
                    .modal(true)
                    .transient_for(&utils::root(self)).build();

                dialog.add_responses(&[
                    ("cancel", &gettext("_Cancel")),
                    ("delete", &gettext("_Force Delete")),
                ]);
                dialog.set_default_response(Some("cancel"));
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

                dialog.connect_response(
                    None,
                    clone!(@weak self as obj, @weak image => move |_, response| {
                        if response == "delete" {
                            obj.delete_image(&image);
                        }
                    }),
                );

                dialog.present();
            } else {
                self.delete_image(image);
            }
        }
    }

    fn delete_image(&self, image: &model::Image) {
        image.delete(clone!(@weak self as obj => move |image, result| {
            if let Err(e) = result {
                utils::show_toast(
                    &obj,
                    // Translators: The first "{}" is a placeholder for the image id, the second is for an error message.
                    &gettext!("Error on deleting image '{}': {}", image.id(), e)
                );
            }
        }));
    }

    pub(crate) fn create_container(&self) {
        if let Some(image) = self.image().as_ref() {
            utils::find_leaflet_overlay(self)
                .show_details(&view::ContainerCreationPage::from(image));
        }
    }
}
