use gettextrs::gettext;
use gtk::glib::{clone, closure, WeakRef};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, CompositeTemplate};
use once_cell::sync::Lazy;

use crate::window::Window;
use crate::{model, utils, view};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image-details-page.ui")]
    pub(crate) struct ImageDetailsPage {
        pub(super) image: WeakRef<model::Image>,
        #[template_child]
        pub(super) leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
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
        pub(super) repo_tags_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) containers_preferences_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageDetailsPage {
        const NAME: &'static str = "ImageDetailsPage";
        type Type = super::ImageDetailsPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("navigation.go-first", None, move |widget, _, _| {
                widget.navigate_to_first();
            });
            klass.install_action("navigation.back", None, move |widget, _, _| {
                widget.navigate_back();
            });
            klass.install_action("image.delete", None, move |widget, _, _| {
                widget.delete();
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
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "image" => {
                    self.image.set(value.get().unwrap());
                }
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
            let image_id_expr = image_expr
                .chain_property::<model::Image>("id")
                .chain_closure::<String>(closure!(|_: glib::Object, id: &str| {
                    id.chars().take(12).collect::<String>()
                }));

            image_id_expr.bind(&*self.window_title, "subtitle", Some(obj));

            image_id_expr.bind(&*self.id_row, "value", Some(obj));

            image_expr
                .chain_property::<model::Image>("created")
                .chain_closure::<String>(closure!(|_: glib::Object, created: i64| {
                    glib::DateTime::from_unix_local(created)
                        .unwrap()
                        .format("%x %X")
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

            let image_config_expr = image_expr.chain_property::<model::Image>("config");

            image_config_expr
                .chain_property::<model::ImageConfig>("cmd")
                .chain_closure::<bool>(closure!(|_: glib::Object, cmd: Option<&str>| {
                    cmd.is_some()
                }))
                .bind(&*self.command_row, "visible", Some(obj));

            image_config_expr
                .chain_property::<model::ImageConfig>("entrypoint")
                .chain_closure::<bool>(closure!(|_: glib::Object, entrypoint: Option<&str>| {
                    entrypoint.is_some()
                }))
                .bind(&*self.entrypoint_row, "visible", Some(obj));

            image_config_expr
                .chain_property::<model::ImageConfig>("exposed-ports")
                .chain_closure::<String>(closure!(
                    |_: glib::Object, exposed_ports: utils::BoxedStringBTreeSet| {
                        utils::format_iter(exposed_ports.iter(), "\n")
                    }
                ))
                .bind(&*self.ports_row, "value", Some(obj));

            image_config_expr
                .chain_property::<model::ImageConfig>("exposed-ports")
                .chain_closure::<bool>(closure!(
                    |_: glib::Object, exposed_ports: utils::BoxedStringBTreeSet| {
                        exposed_ports.len() > 0
                    }
                ))
                .bind(&*self.ports_row, "visible", Some(obj));

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

            let container_list_expr = image_expr.chain_property::<model::Image>("container-list");

            gtk::ClosureExpression::new::<String, _, _>(
                &[
                    container_list_expr.chain_property::<model::SimpleContainerList>("len"),
                    container_list_expr.chain_property::<model::SimpleContainerList>("running"),
                ],
                closure!(|_: glib::Object, len: u32, running: u32| {
                    if len > 0 {
                        gettext!(
                            // Translators: There's a wide space (U+2002) between ", {}".
                            "{} Containers total, {} running",
                            len,
                            running
                        )
                    } else {
                        gettext("There are no containers associated with this image.")
                    }
                }),
            )
            .bind(
                &*self.containers_preferences_group,
                "description",
                Some(obj),
            );

            let sorter = gtk::CustomSorter::new(|obj1, obj2| {
                let container1 = obj1.downcast_ref::<model::Container>().unwrap();
                let container2 = obj2.downcast_ref::<model::Container>().unwrap();

                container1.name().cmp(&container2.name()).into()
            });

            let image = obj.image().unwrap();
            let model = image.container_list();

            model.connect_name_changed(clone!(@weak sorter => move |_, _| {
                glib::timeout_add_seconds_local_once(
                    1,
                    clone!(@weak sorter => move || sorter.changed(gtk::SorterChange::Different)),
                );
            }));

            self.list_box.bind_model(
                Some(&gtk::SortListModel::new(Some(model), Some(&sorter))),
                |item| view::ContainerRow::from(item.downcast_ref().unwrap()).upcast(),
            );

            obj.set_list_box_visibility(model.upcast_ref());
            model.connect_items_changed(clone!(@weak obj => move |model, _, _, _| {
                obj.set_list_box_visibility(model.upcast_ref());
            }));

            image.connect_notify_local(
                Some("to-be-deleted"),
                clone!(@weak obj => move|image, _| {
                    obj.action_set_enabled("image.delete", !image.to_be_deleted());
                }),
            );
        }

        fn dispose(&self, _obj: &Self::Type) {
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
    pub(crate) fn image(&self) -> Option<model::Image> {
        self.imp().image.upgrade()
    }

    fn navigate_to_first(&self) {
        self.root_leaflet_overlay().hide_details();
    }

    fn navigate_back(&self) {
        self.previous_leaflet_overlay().hide_details();
    }

    fn previous_leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::find_leaflet_overlay(self)
    }

    fn root_leaflet_overlay(&self) -> view::LeafletOverlay {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .leaflet_overlay()
    }

    fn delete(&self) {
        self.image().unwrap().delete(
            clone!(@weak self as obj => move |image, result| match result {
                Ok(_) => {
                    obj.imp().stack.set_visible_child_name("deleted");
                    obj.show_toast(
                        // Translators: "{}" is a placeholder for the image id.
                        &gettext!("Successfully deleted image '{}'", image.id())
                    );
                }
                Err(_) => obj.show_toast(
                    // Translators: "{}" is a placeholder for the image id.
                    &gettext!("Error on deleting image '{}'", image.id())
                ),
            }),
        );
    }

    fn show_toast(&self, title: &str) {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .show_toast(
                &adw::Toast::builder()
                    .title(title)
                    .timeout(3)
                    .priority(adw::ToastPriority::High)
                    .build(),
            );
    }

    fn set_list_box_visibility(&self, model: &gio::ListModel) {
        self.imp().list_box.set_visible(model.n_items() > 0);
    }
}
