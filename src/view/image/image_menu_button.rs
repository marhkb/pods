use std::cell::Cell;

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

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image-menu-button.ui")]
    pub(crate) struct ImageMenuButton {
        pub(super) image: WeakRef<model::Image>,
        pub(super) action_ongoing: Cell<bool>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageMenuButton {
        const NAME: &'static str = "ImageMenuButton";
        type Type = super::ImageMenuButton;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("image.create-container", None, move |widget, _, _| {
                widget.create_container();
            });
            klass.install_action("image.delete", None, move |widget, _, _| {
                widget.delete();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageMenuButton {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "image",
                        "Image",
                        "The image of this ImageMenuButton",
                        model::Image::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecBoolean::new(
                        "action-ongoing",
                        "Action Ongoing",
                        "Whether an action (starting, stopping, etc.) is currently ongoing",
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
                "action-ongoing" => obj.set_action_ongoing(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image" => obj.image().to_value(),
                "action-ongoing" => obj.action_ongoing().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            Self::Type::this_expression("css-classes").bind(
                &*self.menu_button,
                "css-classes",
                Some(obj),
            );

            Self::Type::this_expression("action-ongoing")
                .chain_closure::<String>(closure!(|_: glib::Object, action_ongoing: bool| {
                    if action_ongoing {
                        "ongoing"
                    } else {
                        "menu"
                    }
                }))
                .bind(&*self.stack, "visible-child-name", Some(obj));
        }

        fn dispose(&self, obj: &Self::Type) {
            let mut child = obj.first_child();
            while let Some(child_) = child {
                child = child_.next_sibling();
                child_.unparent();
            }
        }
    }

    impl WidgetImpl for ImageMenuButton {}
}

glib::wrapper! {
    pub(crate) struct ImageMenuButton(ObjectSubclass<imp::ImageMenuButton>)
        @extends gtk::Widget;
}

impl ImageMenuButton {
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

    pub(crate) fn action_ongoing(&self) -> bool {
        self.imp().action_ongoing.get()
    }

    pub(crate) fn set_action_ongoing(&self, value: bool) {
        if self.action_ongoing() == value {
            return;
        }
        self.imp().action_ongoing.replace(value);
        self.notify("action-ongoing");
    }

    fn delete(&self) {
        self.set_action_ongoing(true);

        if let Some(image) = self.image().as_ref() {
            image.delete(clone!(@weak self as obj => move |image, result| {
                obj.set_action_ongoing(false);

                utils::show_toast(&obj, &match result {
                    Ok(_) => {
                        // Translators: "{}" is a placeholder for the image id.
                        gettext!("Successfully deleted image '{}'", image.id())
                    }
                    Err(_) => {
                        // Translators: "{}" is a placeholder for the image id.
                        gettext!("Error on deleting image '{}'", image.id())
                    }
                })
            }));
        }
    }

    fn create_container(&self) {
        if let Some(image) = self.image().as_ref() {
            utils::find_leaflet_overlay(self).show_details(&view::ContainerCreationPage::new(
                &image
                    .image_list()
                    .as_ref()
                    .and_then(model::ImageList::client)
                    .unwrap(),
                Some(image),
            ));
        }
    }
}
