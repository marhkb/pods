use std::cell::RefCell;

use gettextrs::gettext;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_INSPECT_IMAGE: &str = "image-details-page.inspect-image";
const ACTION_PULL_LATEST: &str = "image-details-page.pull-latest";
const ACTION_DELETE_IMAGE: &str = "image-details-page.delete-image";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image/details-page.ui")]
    pub(crate) struct DetailsPage {
        pub(super) image: glib::WeakRef<model::Image>,
        pub(super) handler_id: RefCell<Option<glib::SignalHandlerId>>,
        #[template_child]
        pub(super) back_navigation_controls: TemplateChild<view::BackNavigationControls>,
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
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DetailsPage {
        const NAME: &'static str = "PdsImageDetailsPage";
        type Type = super::DetailsPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_INSPECT_IMAGE, None, move |widget, _, _| {
                widget.show_inspection();
            });

            klass.install_action(ACTION_PULL_LATEST, None, move |widget, _, _| {
                widget.pull_latest();
            });

            klass.install_action(ACTION_DELETE_IMAGE, None, move |widget, _, _| {
                widget.delete_image();
            });

            // For displaying a mnemonic.
            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                view::ContainersGroup::action_create_container(),
                None,
            );
            klass.install_action(
                view::ContainersGroup::action_create_container(),
                None,
                move |widget, _, _| {
                    widget.create_container();
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DetailsPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::Image>("image")
                    .flags(
                        glib::ParamFlags::READWRITE
                            | glib::ParamFlags::CONSTRUCT
                            | glib::ParamFlags::EXPLICIT_NOTIFY,
                    )
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "image" => self.instance().set_image(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image" => self.instance().image().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            let image_expr = Self::Type::this_expression("image");

            image_expr
                .chain_property::<model::Image>("to-be-deleted")
                .watch(
                    Some(obj),
                    clone!(@weak obj => move || {
                        obj.action_set_enabled(
                            ACTION_DELETE_IMAGE,
                            obj.image().map(|image| !image.to_be_deleted()).unwrap_or(false),
                        );
                    }),
                );

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

            gtk::ClosureExpression::new::<String>(
                &[
                    Self::Type::this_expression("root")
                        .chain_property::<gtk::Window>("application")
                        .chain_property::<crate::Application>("ticks"),
                    image_expr.chain_property::<model::Image>("created"),
                ],
                closure!(|_: Self::Type, _ticks: u64, created: i64| {
                    utils::format_ago(utils::timespan_now(created))
                }),
            )
            .bind(&*self.created_row, "value", Some(obj));

            gtk::ClosureExpression::new::<String>(
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

        fn dispose(&self) {
            utils::ChildIter::from(&*self.instance()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for DetailsPage {}
}

glib::wrapper! {
    pub(crate) struct DetailsPage(ObjectSubclass<imp::DetailsPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Image> for DetailsPage {
    fn from(image: &model::Image) -> Self {
        glib::Object::new::<Self>(&[("image", image)])
    }
}

impl DetailsPage {
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
            image.inspect(clone!(@weak self as obj => move |e| {
                utils::show_error_toast(&obj, &gettext("Error on loading image details"), &e.to_string());
            }));

            let handler_id = image.connect_deleted(clone!(@weak self as obj => move |image| {
                utils::show_toast(&obj, &gettext!("Image '{}' has been deleted", image.id()));
                obj.imp().back_navigation_controls.navigate_back();
            }));
            imp.handler_id.replace(Some(handler_id));
        }

        imp.image.set(value);
        self.notify("image");
    }

    fn show_inspection(&self) {
        super::show_inspection(&*self.imp().leaflet_overlay, self.image());
    }

    fn pull_latest(&self) {
        super::pull_latest(Some(&*self.imp().leaflet_overlay), self.image());
    }

    fn delete_image(&self) {
        super::delete_image_show_confirmation(self.upcast_ref(), self.image());
    }

    fn create_container(&self) {
        super::create_container(&*self.imp().leaflet_overlay, self.image());
    }
}
