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

const ACTION_TAG: &str = "image-details-page.tag";
const ACTION_INSPECT_IMAGE: &str = "image-details-page.inspect-image";
const ACTION_SHOW_HISTORY: &str = "image-details-page.show-history";
const ACTION_DELETE_IMAGE: &str = "image-details-page.delete-image";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image/details-page.ui")]
    pub(crate) struct DetailsPage {
        pub(super) image: glib::WeakRef<model::Image>,
        pub(super) handler_id: RefCell<Option<glib::SignalHandlerId>>,
        #[template_child]
        pub(super) create_tag_row: TemplateChild<gtk::ListBoxRow>,
        #[template_child]
        pub(super) back_navigation_controls: TemplateChild<view::BackNavigationControls>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) inspection_spinner: TemplateChild<gtk::Spinner>,
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
        pub(super) repo_tags_list_box: TemplateChild<gtk::ListBox>,
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

            klass.install_action(ACTION_TAG, None, move |widget, _, _| {
                widget.tag();
            });

            klass.install_action(ACTION_INSPECT_IMAGE, None, move |widget, _, _| {
                widget.show_inspection();
            });

            klass.install_action(ACTION_SHOW_HISTORY, None, move |widget, _, _| {
                widget.show_history();
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
                    .construct()
                    .explicit_notify()
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "image" => self.obj().set_image(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image" => self.obj().image().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let image_expr = Self::Type::this_expression("image");
            let data_expr = image_expr.chain_property::<model::Image>("data");
            let image_config_expr = data_expr.chain_property::<model::ImageData>("config");
            let cmd_expr = image_config_expr.chain_property::<model::ImageConfig>("cmd");
            let entrypoint_expr =
                image_config_expr.chain_property::<model::ImageConfig>("entrypoint");
            let exposed_ports_expr =
                image_config_expr.chain_property::<model::ImageConfig>("exposed-ports");

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

            data_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, cmd: Option<model::ImageData>| {
                    cmd.is_none()
                }))
                .bind(&*self.inspection_spinner, "visible", Some(obj));

            image_expr
                .chain_property::<model::Image>("id")
                .chain_closure::<String>(closure!(|_: Self::Type, id: &str| utils::format_id(id)))
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
                    |_: Self::Type, size: u64, shared_size: u64, virtual_size: u64| {
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

            cmd_expr.bind(&*self.command_row, "value", Some(obj));
            cmd_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, cmd: Option<&str>| {
                    cmd.is_some()
                }))
                .bind(&*self.command_row, "visible", Some(obj));

            entrypoint_expr.bind(&*self.entrypoint_row, "value", Some(obj));
            entrypoint_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, entrypoint: Option<&str>| {
                    entrypoint.is_some()
                }))
                .bind(&*self.entrypoint_row, "visible", Some(obj));

            exposed_ports_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, exposed_ports: gtk::StringList| {
                        let exposed_ports = exposed_ports
                            .iter::<glib::Object>()
                            .unwrap()
                            .map(|obj| {
                                obj.unwrap()
                                    .downcast::<gtk::StringObject>()
                                    .unwrap()
                                    .string()
                            })
                            .collect::<Vec<_>>();

                        utils::format_iter(exposed_ports.iter().map(glib::GString::as_str), ", ")
                    }
                ))
                .bind(&*self.ports_row, "value", Some(obj));

            exposed_ports_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, exposed_ports: gtk::StringList| {
                    exposed_ports.n_items() > 0
                }))
                .bind(&*self.ports_row, "visible", Some(obj));
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
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
        glib::Object::builder::<Self>()
            .property("image", image)
            .build()
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

        imp.window_title.set_subtitle("");
        if let Some(image) = self.image() {
            image.disconnect(imp.handler_id.take().unwrap());
        }
        imp.repo_tags_list_box.unbind_model();

        if let Some(image) = value {
            imp.window_title.set_subtitle(&utils::format_id(image.id()));
            image.inspect(clone!(@weak self as obj => move |e| {
                utils::show_error_toast(&obj, &gettext("Error on loading image details"), &e.to_string());
            }));

            let handler_id = image.connect_deleted(clone!(@weak self as obj => move |image| {
                utils::show_toast(&obj, &gettext!("Image '{}' has been deleted", image.id()));
                obj.imp().back_navigation_controls.navigate_back();
            }));
            imp.handler_id.replace(Some(handler_id));

            let model = gtk::SortListModel::new(
                Some(image.repo_tags()),
                Some(&gtk::StringSorter::new(Some(
                    model::RepoTag::this_expression("full"),
                ))),
            );
            imp.repo_tags_list_box.bind_model(Some(&model), |tag| {
                let repo_tag = tag.downcast_ref::<model::RepoTag>().unwrap();
                view::RepoTagRow::from(repo_tag).upcast()
            });
            imp.repo_tags_list_box.append(&*imp.create_tag_row);
        }

        imp.image.set(value);
        self.notify("image");
    }

    fn tag(&self) {
        if let Some(image) = self.image() {
            let dialog = view::RepoTagAddDialog::from(&image);
            dialog.set_transient_for(Some(&utils::root(self)));
            dialog.present();
        }
    }

    fn show_inspection(&self) {
        if let Some(image) = self.image() {
            let weak_ref = glib::WeakRef::new();
            weak_ref.set(Some(&image));

            self.imp()
                .leaflet_overlay
                .show_details(&view::SourceViewPage::from(view::Entity::Image(weak_ref)));
        }
    }

    fn show_history(&self) {
        if let Some(image) = self.image() {
            self.imp()
                .leaflet_overlay
                .show_details(&view::ImageHistoryPage::from(&image));
        }
    }

    fn delete_image(&self) {
        super::delete_image_show_confirmation(self.upcast_ref(), self.image());
    }

    fn create_container(&self) {
        super::create_container(self, self.image());
    }
}
