use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::model;

mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::{ExpanderRowImpl, PreferencesRowImpl};
    use gettextrs::gettext;
    use gtk::glib::{clone, closure};
    use gtk::CompositeTemplate;
    use once_cell::sync::Lazy;

    use super::*;
    use crate::{utils, view};

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/image-row.ui")]
    pub struct ImageRow {
        pub image: RefCell<Option<model::Image>>,
        #[template_child]
        pub id_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub created_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub size_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub containers_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub command_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub entrypoint_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub ports_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub repo_tags_row: TemplateChild<view::PropertyRow>,
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
                        // TODO: Show a toast notification
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
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
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
            repo_tags_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: glib::Object, repo_tags: utils::BoxedStringVec| {
                        repo_tags
                            .iter()
                            .next()
                            .map(|_| None)
                            .unwrap_or_else(|| Some(glib::GString::from("image-tag-none")))
                            .iter()
                            .chain(css_classes.iter())
                            .cloned()
                            .collect::<Vec<_>>()
                    }
                ))
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
                                    "{} (Virtual: {})",
                                    formatted_size,
                                    glib::format_size(virtual_size),
                                )
                            }
                        } else if size == virtual_size {
                            if shared_size > 0 {
                                gettext!(
                                    "{} (Shared: {})",
                                    formatted_size,
                                    glib::format_size(shared_size),
                                )
                            } else {
                                formatted_size.to_string()
                            }
                        } else {
                            gettext!(
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
    pub struct ImageRow(ObjectSubclass<imp::ImageRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ExpanderRow;
}

impl From<&model::Image> for ImageRow {
    fn from(image: &model::Image) -> Self {
        glib::Object::new(&[("image", image)]).expect("Failed to create ImageRow")
    }
}

impl ImageRow {
    pub fn image(&self) -> Option<model::Image> {
        self.imp().image.borrow().clone()
    }
}
