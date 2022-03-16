use std::cell::RefCell;

use adw::subclass::prelude::{ExpanderRowImpl, PreferencesRowImpl};
use gettextrs::gettext;
use gtk::glib::{clone, closure};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use once_cell::sync::Lazy;

use crate::window::Window;
use crate::{model, utils, view};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/image-row.ui")]
    pub(crate) struct ImageRow {
        pub(super) image: RefCell<Option<model::Image>>,
        #[template_child]
        pub(super) id_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) created_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) size_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) containers_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) command_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) entrypoint_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) ports_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) repo_tags_row: TemplateChild<view::PropertyRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageRow {
        const NAME: &'static str = "ImageRow";
        type Type = super::ImageRow;
        type ParentType = adw::ExpanderRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("image.delete", None, move |widget, _, _| {
                widget.action_set_enabled("image.delete", false);
                widget
                    .image()
                    .unwrap()
                    .delete(clone!(@weak widget => move |_| {
                        widget.action_set_enabled("image.delete", true);
                        widget.root().unwrap().downcast::<Window>().unwrap().show_toast(
                            &adw::Toast::builder()
                                .title(&gettext("Error on deleting image"))
                                .timeout(3)
                                .priority(adw::ToastPriority::High)
                                .build()
                        );
                    }));
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "image",
                    "Image",
                    "The image of this ImageRow",
                    model::Image::static_type(),
                    glib::ParamFlags::READWRITE,
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
                    self.image.replace(value.get().unwrap());
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
            let image_config_expr = image_expr.chain_property::<model::Image>("config");
            let repo_tags_expr = image_expr.chain_property::<model::Image>("repo-tags");

            repo_tags_expr
                .chain_closure::<String>(closure!(
                    |_: glib::Object, repo_tags: utils::BoxedStringVec| {
                        utils::escape(&utils::format_option(repo_tags.iter().next()))
                    }
                ))
                .bind(obj, "title", Some(obj));

            let css_classes = obj.css_classes();
            gtk::ClosureExpression::new::<Vec<String>, _, _>(
                &[
                    repo_tags_expr,
                    image_expr.chain_property::<model::Image>("to-be-deleted"),
                ],
                closure!(|_: glib::Object,
                          repo_tags: utils::BoxedStringVec,
                          to_be_deleted: bool| {
                    repo_tags
                        .iter()
                        .next()
                        .map(|_| None)
                        .unwrap_or_else(|| Some(glib::GString::from("image-tag-none")))
                        .into_iter()
                        .chain(if to_be_deleted {
                            Some(glib::GString::from("image-to-be-deleted"))
                        } else {
                            None
                        })
                        .chain(css_classes.iter().cloned())
                        .collect::<Vec<_>>()
                }),
            )
            .bind(obj, "css-classes", Some(obj));

            image_expr
                .chain_property::<model::Image>("id")
                .chain_closure::<String>(closure!(|_: glib::Object, id: &str| {
                    id.chars().take(12).collect::<String>()
                }))
                .bind(obj, "subtitle", Some(obj));

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

            image_expr
                .chain_property::<model::Image>("containers")
                .chain_closure::<String>(closure!(|_: glib::Object, containers: u64| {
                    // Translators: "{}" is placeholder for an integer value.
                    gettext!("By {} containers", containers)
                }))
                .bind(&*self.containers_row, "value", Some(obj));

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
                        utils::format_iter(exposed_ports.0.iter(), "\n")
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
        }
    }

    impl WidgetImpl for ImageRow {}
    impl ListBoxRowImpl for ImageRow {}
    impl PreferencesRowImpl for ImageRow {}
    impl ExpanderRowImpl for ImageRow {}
}

glib::wrapper! {
    pub(crate) struct ImageRow(ObjectSubclass<imp::ImageRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ExpanderRow;
}

impl From<&model::Image> for ImageRow {
    fn from(image: &model::Image) -> Self {
        glib::Object::new(&[("image", image)]).expect("Failed to create ImageRow")
    }
}

impl ImageRow {
    pub(crate) fn image(&self) -> Option<model::Image> {
        self.imp().image.borrow().clone()
    }
}
