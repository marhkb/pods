use adw::subclass::prelude::ActionRowImpl;
use adw::subclass::prelude::PreferencesRowImpl;
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
    #[template(resource = "/com/github/marhkb/Pods/ui/image-row.ui")]
    pub(crate) struct ImageRow {
        pub(super) image: WeakRef<model::Image>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageRow {
        const NAME: &'static str = "ImageRow";
        type Type = super::ImageRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("image.show-details", None, move |widget, _, _| {
                widget.show_details();
            });

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

    impl ObjectImpl for ImageRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "image",
                    "Image",
                    "The image of this ImageRow",
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

            self.menu_button
                .set_menu_model(Some(&super::super::image_menu()));

            let image_expr = Self::Type::this_expression("image");
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

            obj.image().unwrap().connect_notify_local(
                Some("to-be-deleted"),
                clone!(@weak obj => move|image, _| {
                    obj.action_set_enabled("image.show-details", !image.to_be_deleted());
                    obj.action_set_enabled("image.delete", !image.to_be_deleted());
                }),
            );
        }
    }

    impl WidgetImpl for ImageRow {}
    impl ListBoxRowImpl for ImageRow {}
    impl PreferencesRowImpl for ImageRow {}
    impl ActionRowImpl for ImageRow {}
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
        self.imp().image.upgrade()
    }

    fn show_details(&self) {
        utils::find_leaflet_overlay(self)
            .show_details(&view::ImageDetailsPage::from(&self.image().unwrap()));
    }

    fn create_container(&self) {
        if let Some(image) = self.image().as_ref() {
            super::create_container(
                self.upcast_ref(),
                &image
                    .image_list()
                    .as_ref()
                    .and_then(model::ImageList::client)
                    .unwrap(),
                Some(image),
            );
        }
    }

    fn delete(&self) {
        if let Some(image) = self.image().as_ref() {
            super::delete(self.upcast_ref(), image);
        }
    }
}
