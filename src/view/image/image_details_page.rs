use std::cell::RefCell;

use adw::traits::BinExt;
use gettextrs::gettext;
use gtk::gdk;
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
use crate::window::Window;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image-details-page.ui")]
    pub(crate) struct ImageDetailsPage {
        pub(super) image: WeakRef<model::Image>,
        pub(super) handler_id: RefCell<Option<glib::SignalHandlerId>>,
        #[template_child]
        pub(super) leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub(super) menu_button: TemplateChild<view::ImageMenuButton>,
        #[template_child]
        pub(super) repo_tags_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) id_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) created_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) size_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) command_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) entrypoint_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) ports_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) inspection_row: TemplateChild<adw::PreferencesRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageDetailsPage {
        const NAME: &'static str = "ImageDetailsPage";
        type Type = super::ImageDetailsPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.add_binding_action(gdk::Key::F10, gdk::ModifierType::empty(), "menu.show", None);
            klass.install_action("menu.show", None, |widget, _, _| {
                widget.show_menu();
            });

            klass.install_action("navigation.go-first", None, move |widget, _, _| {
                widget.navigate_to_first();
            });
            klass.install_action("navigation.back", None, move |widget, _, _| {
                widget.navigate_back();
            });

            klass.install_action("image.inspect", None, move |widget, _, _| {
                widget.show_inspection();
            });

            // For displaying a mnemonic.
            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                "image.create-container",
                None,
            );

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                "containers.create",
                None,
            );
            klass.install_action("containers.create", None, move |widget, _, _| {
                widget.create_container();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageDetailsPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "image",
                    "Image",
                    "The image of this ImageDetailsPage",
                    model::Image::static_type(),
                    glib::ParamFlags::READWRITE
                        | glib::ParamFlags::CONSTRUCT
                        | glib::ParamFlags::READWRITE
                        | glib::ParamFlags::EXPLICIT_NOTIFY,
                )]
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
                "image" => obj.set_image(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image" => obj.image().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let image_expr = Self::Type::this_expression("image");

            image_expr
                .chain_property::<model::Image>("repo-tags")
                .chain_closure::<String>(closure!(
                    |_: glib::Object, repo_tags: utils::BoxedStringVec| {
                        utils::format_iter(&mut repo_tags.iter(), "; ")
                    }
                ))
                .bind(&*self.repo_tags_row, "value", Some(obj));

            image_expr
                .chain_property::<model::Image>("repo-tags")
                .chain_closure::<bool>(closure!(
                    |_: glib::Object, repo_tags: utils::BoxedStringVec| { repo_tags.len() > 0 }
                ))
                .bind(&*self.repo_tags_row, "visible", Some(obj));

            image_expr
                .chain_property::<model::Image>("id")
                .chain_closure::<String>(closure!(|_: glib::Object, id: &str| {
                    id.chars().take(12).collect::<String>()
                }))
                .bind(&*self.id_row, "value", Some(obj));

            image_expr
                .chain_property::<model::Image>("created")
                .chain_closure::<String>(closure!(|_: glib::Object, created: i64| {
                    glib::DateTime::from_unix_local(created)
                        .unwrap()
                        .format(
                            // Translators: This is a date time format (https://valadoc.org/glib-2.0/GLib.DateTime.format.html)
                            &gettext("%x %X"),
                        )
                        .unwrap()
                }))
                .bind(&*self.created_row, "value", Some(obj));

            gtk::ClosureExpression::new::<String, _, _>(
                &[
                    image_expr.chain_property::<model::Image>("size").upcast(),
                    image_expr
                        .chain_property::<model::Image>("shared-size")
                        .upcast(),
                    image_expr
                        .chain_property::<model::Image>("virtual-size")
                        .upcast(),
                ],
                closure!(
                    |_: glib::Object, size: u64, shared_size: u64, virtual_size: u64| {
                        let formatted_size = glib::format_size(size);
                        if size == shared_size {
                            if shared_size == virtual_size {
                                formatted_size.to_string()
                            } else {
                                gettext!(
                                    // Translators: "{}" are placeholders for storage space.
                                    "{} (Virtual: {})",
                                    formatted_size,
                                    glib::format_size(virtual_size),
                                )
                            }
                        } else if size == virtual_size {
                            if shared_size > 0 {
                                gettext!(
                                    // Translators: "{}" are placeholders for storage space.
                                    "{} (Shared: {})",
                                    formatted_size,
                                    glib::format_size(shared_size),
                                )
                            } else {
                                formatted_size.to_string()
                            }
                        } else {
                            gettext!(
                                // Translators: "{}" are placeholders for storage space.
                                "{} (Shared: {}, Virtual: {})",
                                formatted_size,
                                glib::format_size(shared_size),
                                glib::format_size(virtual_size),
                            )
                        }
                    }
                ),
            )
            .bind(&*self.size_row, "value", Some(obj));

            let data_expr = image_expr.chain_property::<model::Image>("data");
            let image_config_expr = data_expr.chain_property::<model::ImageData>("config");
            let cmd_expr = image_config_expr.chain_property::<model::ImageConfig>("cmd");
            let entrypoint_expr =
                image_config_expr.chain_property::<model::ImageConfig>("entrypoint");
            let exposed_ports_expr =
                image_config_expr.chain_property::<model::ImageConfig>("exposed-ports");

            cmd_expr.bind(&*self.command_row, "value", Some(obj));
            cmd_expr
                .chain_closure::<bool>(closure!(|_: glib::Object, cmd: Option<&str>| {
                    cmd.is_some()
                }))
                .bind(&*self.command_row, "visible", Some(obj));

            entrypoint_expr.bind(&*self.entrypoint_row, "value", Some(obj));
            entrypoint_expr
                .chain_closure::<bool>(closure!(|_: glib::Object, entrypoint: Option<&str>| {
                    entrypoint.is_some()
                }))
                .bind(&*self.entrypoint_row, "visible", Some(obj));

            exposed_ports_expr
                .chain_closure::<String>(closure!(
                    |_: glib::Object, exposed_ports: utils::BoxedStringBTreeSet| {
                        utils::format_iter(exposed_ports.iter(), "\n")
                    }
                ))
                .bind(&*self.ports_row, "value", Some(obj));

            exposed_ports_expr
                .chain_closure::<bool>(closure!(
                    |_: glib::Object, exposed_ports: utils::BoxedStringBTreeSet| {
                        exposed_ports.len() > 0
                    }
                ))
                .bind(&*self.ports_row, "visible", Some(obj));

            data_expr
                .chain_closure::<bool>(closure!(
                    |_: glib::Object, cmd: Option<model::ImageData>| { cmd.is_none() }
                ))
                .bind(&*self.inspection_row, "visible", Some(obj));
        }

        fn dispose(&self, obj: &Self::Type) {
            if let Some(image) = obj.image() {
                image.disconnect(self.handler_id.take().unwrap());
            }
            self.leaflet.unparent();
        }
    }

    impl WidgetImpl for ImageDetailsPage {
        fn realize(&self, widget: &Self::Type) {
            self.parent_realize(widget);

            widget.action_set_enabled(
                "navigation.go-first",
                widget.previous_leaflet_overlay() != widget.root_leaflet_overlay(),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageDetailsPage(ObjectSubclass<imp::ImageDetailsPage>) @extends gtk::Widget;
}

impl From<&model::Image> for ImageDetailsPage {
    fn from(image: &model::Image) -> Self {
        glib::Object::new(&[("image", image)]).expect("Failed to create ImageDetailsPage")
    }
}

impl ImageDetailsPage {
    fn show_menu(&self) {
        let imp = self.imp();
        if utils::leaflet_overlay(&imp.leaflet).child().is_none() {
            imp.menu_button.popup();
        }
    }

    pub(crate) fn image(&self) -> Option<model::Image> {
        self.imp().image.upgrade()
    }

    pub(crate) fn set_image(&self, value: Option<&model::Image>) {
        if self.image().as_ref() == value {
            return;
        }

        let imp = self.imp();

        if let Some(image) = self.image() {
            image.disconnect(imp.handler_id.take().unwrap());
        }

        if let Some(image) = value {
            image.inspect();
            image.connect_inspection_failed(clone!(@weak self as obj => move |_| {
                utils::show_toast(&obj, &gettext("Error on loading image details"));
            }));

            let handler_id = image.connect_deleted(clone!(@weak self as obj => move |image| {
                utils::show_toast(&obj, &gettext!("Image '{}' has been deleted", image.id()));
                obj.navigate_back();
            }));
            imp.handler_id.replace(Some(handler_id));
        }

        imp.image.set(value);
        self.notify("image");
    }

    fn navigate_to_first(&self) {
        self.root_leaflet_overlay().hide_details();
    }

    fn navigate_back(&self) {
        self.previous_leaflet_overlay().hide_details();
    }

    fn previous_leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::find_parent_leaflet_overlay(self)
    }

    fn root_leaflet_overlay(&self) -> view::LeafletOverlay {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .leaflet_overlay()
    }

    fn show_inspection(&self) {
        if let Some(image) = self.image().as_ref().and_then(model::Image::api_image) {
            self.action_set_enabled("image.inspect", false);
            utils::do_async(
                async move { image.inspect().await.map_err(anyhow::Error::from) },
                clone!(@weak self as obj => move |result| {
                    obj.action_set_enabled("image.inspect", true);
                    match result
                        .and_then(|data| view::InspectionPage::new(
                            &gettext("Image Inspection"), &data
                        ))
                    {
                        Ok(page) => utils::leaflet_overlay(&*obj.imp().leaflet).show_details(&page),
                        Err(e) => utils::show_error_toast(
                            &obj,
                            &gettext("Error on inspecting image"),
                            &e.to_string()
                        ),
                    }
                }),
            );
        }
    }

    fn create_container(&self) {
        let imp = self.imp();

        if utils::leaflet_overlay(&*imp.leaflet).child().is_none() {
            imp.menu_button
                .activate_action("image.create-container", None)
                .unwrap();
        }
    }
}
