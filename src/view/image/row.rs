use std::cell::RefCell;

use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::model::SelectableExt;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image/row.ui")]
    pub(crate) struct Row {
        pub(super) image: glib::WeakRef<model::Image>,
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[template_child]
        pub(super) check_button_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) repo_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) id_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) tag_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) end_box_revealer: TemplateChild<gtk::Revealer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsImageRow";
        type Type = super::Row;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("image-row.activate", None, move |widget, _, _| {
                widget.activate();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Row {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::Image>("image")
                    .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT)
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

            let selection_mode_expr = image_expr
                .chain_property::<model::Image>("image-list")
                .chain_property::<model::ImageList>("selection-mode");

            selection_mode_expr.bind(&*self.check_button_revealer, "reveal-child", Some(obj));
            selection_mode_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, is_selection_mode: bool| {
                    !is_selection_mode
                }))
                .bind(&*self.end_box_revealer, "reveal-child", Some(obj));

            let repo_tags_expr = image_expr.chain_property::<model::Image>("repo-tags");

            gtk::ClosureExpression::new::<String>(
                [
                    repo_tags_expr.upcast_ref(),
                    image_expr
                        .chain_property::<model::Image>("to-be-deleted")
                        .upcast_ref(),
                ],
                closure!(
                    |_: Self::Type, repo_tags: gtk::StringList, to_be_deleted: bool| {
                        let repo = repo_tags.string(0).and_then(|repo_tag| {
                            repo_tag.split_once(':').map(|(name, _)| name.to_string())
                        });
                        let repo = utils::escape(&utils::format_option(repo));

                        if to_be_deleted {
                            format!("<s>{repo}</s>")
                        } else {
                            repo
                        }
                    }
                ),
            )
            .bind(&*self.repo_label, "label", Some(obj));

            let css_classes = self.repo_label.css_classes();
            repo_tags_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, repo_tags: gtk::StringList| {
                        repo_tags
                            .string(0)
                            .map(|_| None)
                            .unwrap_or_else(|| Some(glib::GString::from("dim-label")))
                            .into_iter()
                            .chain(css_classes.iter().cloned())
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.repo_label, "css-classes", Some(obj));

            image_expr
                .chain_property::<model::Image>("id")
                .chain_closure::<String>(closure!(|_: Self::Type, id: &str| {
                    id.chars().take(12).collect::<String>()
                }))
                .bind(&*self.id_label, "label", Some(obj));

            repo_tags_expr
                .chain_closure::<String>(closure!(|_: Self::Type, repo_tags: gtk::StringList| {
                    repo_tags
                        .string(0)
                        .and_then(|repo_tag| {
                            repo_tag.split_once(':').map(|(_, tag)| tag.to_string())
                        })
                        .unwrap_or_default()
                }))
                .bind(&*self.tag_label, "label", Some(obj));

            if let Some(image) = obj.image() {
                obj.action_set_enabled("image.show-details", !image.to_be_deleted());
                image.connect_notify_local(
                    Some("to-be-deleted"),
                    clone!(@weak obj => move|image, _| {
                        obj.action_set_enabled("image.show-details", !image.to_be_deleted());
                    }),
                );
            }
        }
    }

    impl WidgetImpl for Row {}
    impl ListBoxRowImpl for Row {}
}

glib::wrapper! {
    pub(crate) struct Row(ObjectSubclass<imp::Row>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::Actionable, gtk::ConstraintTarget;

}

impl From<&model::Image> for Row {
    fn from(image: &model::Image) -> Self {
        glib::Object::builder::<Self>()
            .property("image", image)
            .build()
    }
}

impl Row {
    pub(crate) fn image(&self) -> Option<model::Image> {
        self.imp().image.upgrade()
    }

    pub(crate) fn set_image(&self, value: Option<&model::Image>) {
        if self.image().as_ref() == value {
            return;
        }

        let imp = self.imp();

        let mut bindings = imp.bindings.borrow_mut();
        while let Some(binding) = bindings.pop() {
            binding.unbind();
        }

        if let Some(image) = value {
            let binding = image
                .bind_property("selected", &*imp.check_button, "active")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            bindings.push(binding);
        }

        imp.image.set(value);
        self.notify("image")
    }

    fn activate(&self) {
        if let Some(image) = self.image().as_ref() {
            if image
                .image_list()
                .map(|list| list.is_selection_mode())
                .unwrap_or(false)
            {
                image.select();
            } else {
                utils::find_leaflet_overlay(self)
                    .show_details(&view::ImageDetailsPage::from(image));
            }
        }
    }
}
