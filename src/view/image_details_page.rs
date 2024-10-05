use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;
use crate::widget;

const ACTION_TAG: &str = "image-details-page.tag";
const ACTION_INSPECT_IMAGE: &str = "image-details-page.inspect-image";
const ACTION_SHOW_HISTORY: &str = "image-details-page.show-history";
const ACTION_DELETE_IMAGE: &str = "image-details-page.delete-image";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageDetailsPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_details_page.ui")]
    pub(crate) struct ImageDetailsPage {
        pub(super) handler_id: RefCell<Option<glib::SignalHandlerId>>,
        #[property(get, set = Self::set_image, construct, nullable)]
        pub(super) image: glib::WeakRef<model::Image>,
        #[template_child]
        pub(super) create_tag_row: TemplateChild<gtk::ListBoxRow>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) inspection_spinner: TemplateChild<adw::Spinner>,
        #[template_child]
        pub(super) id_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) created_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) size_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) command_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) entrypoint_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) ports_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) repo_tags_list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageDetailsPage {
        const NAME: &'static str = "PdsImageDetailsPage";
        type Type = super::ImageDetailsPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_TAG, None, |widget, _, _| {
                widget.tag();
            });

            klass.install_action(ACTION_INSPECT_IMAGE, None, |widget, _, _| {
                widget.show_inspection();
            });

            klass.install_action(ACTION_SHOW_HISTORY, None, |widget, _, _| {
                widget.show_history();
            });

            klass.install_action(ACTION_DELETE_IMAGE, None, |widget, _, _| {
                widget.delete_image();
            });

            // For displaying a mnemonic.
            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                view::ContainersGroup::action_create_container(),
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

    impl ObjectImpl for ImageDetailsPage {
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
                    clone!(
                        #[weak]
                        obj,
                        move || {
                            obj.action_set_enabled(
                                ACTION_DELETE_IMAGE,
                                obj.image()
                                    .map(|image| !image.to_be_deleted())
                                    .unwrap_or(false),
                            );
                        }
                    ),
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
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ImageDetailsPage {}

    impl ImageDetailsPage {
        pub(super) fn set_image(&self, value: Option<&model::Image>) {
            let obj = &*self.obj();
            if obj.image().as_ref() == value {
                return;
            }

            self.window_title.set_subtitle("");
            if let Some(image) = obj.image() {
                image.disconnect(self.handler_id.take().unwrap());
            }
            self.repo_tags_list_box.unbind_model();

            if let Some(image) = value {
                self.window_title
                    .set_subtitle(&utils::format_id(&image.id()));
                image.inspect(clone!(
                    #[weak]
                    obj,
                    move |result| if let Err(e) = result {
                        utils::show_error_toast(
                            &obj,
                            &gettext("Error on loading image details"),
                            &e.to_string(),
                        );
                    }
                ));

                let handler_id = image.connect_deleted(clone!(
                    #[weak]
                    obj,
                    move |image| {
                        utils::show_toast(
                            &obj,
                            gettext!("Image '{}' has been deleted", image.id()),
                        );
                        utils::navigation_view(&obj).pop();
                    }
                ));
                self.handler_id.replace(Some(handler_id));

                let model = gtk::SortListModel::new(
                    Some(image.repo_tags()),
                    Some(gtk::StringSorter::new(Some(
                        model::RepoTag::this_expression("full"),
                    ))),
                );
                self.repo_tags_list_box.bind_model(Some(&model), |tag| {
                    let repo_tag = tag.downcast_ref::<model::RepoTag>().unwrap();
                    view::RepoTagRow::from(repo_tag).upcast()
                });
                self.repo_tags_list_box.append(&*self.create_tag_row);
            }

            self.image.set(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageDetailsPage(ObjectSubclass<imp::ImageDetailsPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Image> for ImageDetailsPage {
    fn from(image: &model::Image) -> Self {
        glib::Object::builder().property("image", image).build()
    }
}

impl ImageDetailsPage {
    fn tag(&self) {
        self.exec_action(|| {
            if let Some(image) = self.image() {
                let dialog = view::RepoTagAddDialog::from(&image);
                dialog.set_transient_for(Some(&utils::root(self)));
                dialog.present();
            }
        });
    }

    fn show_inspection(&self) {
        self.exec_action(|| {
            if let Some(image) = self.image() {
                let weak_ref = glib::WeakRef::new();
                weak_ref.set(Some(&image));

                utils::navigation_view(self).push(
                    &adw::NavigationPage::builder()
                        .child(&view::ScalableTextViewPage::from(view::Entity::Image(
                            weak_ref,
                        )))
                        .build(),
                );
            }
        });
    }

    fn show_history(&self) {
        self.exec_action(|| {
            if let Some(image) = self.image() {
                utils::navigation_view(self).push(
                    &adw::NavigationPage::builder()
                        .child(&view::ImageHistoryPage::from(&image))
                        .build(),
                );
            }
        });
    }

    fn delete_image(&self) {
        self.exec_action(|| {
            view::image::delete_image_show_confirmation(self, self.image());
        });
    }

    fn create_container(&self) {
        self.exec_action(|| {
            view::image::create_container(self, self.image());
        });
    }

    fn exec_action<F: Fn()>(&self, op: F) {
        if utils::navigation_view(self)
            .visible_page()
            .filter(|page| page.child().as_ref() == Some(self.upcast_ref()))
            .is_some()
        {
            op();
        }
    }
}
