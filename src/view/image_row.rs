use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::model::SelectableExt;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_row.ui")]
    pub(crate) struct ImageRow {
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[property(get, set = Self::set_image, construct, nullable)]
        pub(super) image: glib::WeakRef<model::Image>,
        #[template_child]
        pub(super) check_button_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) id_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) repo_tags_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) end_box_revealer: TemplateChild<gtk::Revealer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageRow {
        const NAME: &'static str = "PdsImageRow";
        type Type = super::ImageRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("image-row.activate", None, |widget, _, _| {
                widget.activate();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageRow {
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

            let selection_mode_expr = image_expr
                .chain_property::<model::Image>("image-list")
                .chain_property::<model::ImageList>("selection-mode");

            selection_mode_expr.bind(&*self.check_button_revealer, "reveal-child", Some(obj));
            selection_mode_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, is_selection_mode: bool| {
                    !is_selection_mode
                }))
                .bind(&*self.end_box_revealer, "reveal-child", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [
                    image_expr
                        .chain_property::<model::Image>("id")
                        .chain_closure::<String>(closure!(|_: Self::Type, id: &str| {
                            utils::format_id(id)
                        }))
                        .upcast_ref(),
                    image_expr
                        .chain_property::<model::Image>("to-be-deleted")
                        .upcast_ref(),
                ],
                closure!(|_: Self::Type, id: String, to_be_deleted: bool| {
                    if to_be_deleted {
                        format!("<s>{id}</s>")
                    } else {
                        id
                    }
                }),
            )
            .bind(&*self.id_label, "label", Some(obj));

            let css_classes = utils::css_classes(&*self.id_label);
            image_expr
                .chain_property::<model::Image>("repo-tags")
                .chain_property::<model::RepoTagList>("len")
                .chain_closure::<Vec<String>>(closure!(|_: Self::Type, len: u32| {
                    css_classes
                        .iter()
                        .cloned()
                        .chain(if len == 0 {
                            Some(String::from("dim-label"))
                        } else {
                            None
                        })
                        .collect::<Vec<_>>()
                }))
                .bind(&*self.id_label, "css-classes", Some(obj));

            if let Some(image) = obj.image() {
                obj.action_set_enabled("image.show-details", !image.to_be_deleted());
                image.connect_notify_local(
                    Some("to-be-deleted"),
                    clone!(
                        #[weak]
                        obj,
                        move |image, _| {
                            obj.action_set_enabled("image.show-details", !image.to_be_deleted());
                        }
                    ),
                );
            }
        }
    }

    impl WidgetImpl for ImageRow {}
    impl ListBoxRowImpl for ImageRow {}

    impl ImageRow {
        pub(super) fn set_image(&self, value: Option<&model::Image>) {
            let obj = &*self.obj();
            if obj.image().as_ref() == value {
                return;
            }

            let mut bindings = self.bindings.borrow_mut();
            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }
            self.repo_tags_list_box.unbind_model();

            if let Some(image) = value {
                let binding = image
                    .bind_property("selected", &*self.check_button, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();

                bindings.push(binding);

                let model = gtk::SortListModel::new(
                    Some(image.repo_tags()),
                    Some(gtk::StringSorter::new(Some(
                        model::RepoTag::this_expression("full"),
                    ))),
                );
                self.repo_tags_list_box.bind_model(Some(&model), |tag| {
                    let repo_tag = tag.downcast_ref::<model::RepoTag>().unwrap();
                    view::RepoTagSimpleRow::from(repo_tag).upcast()
                });
            }

            self.image.set(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageRow(ObjectSubclass<imp::ImageRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::Actionable, gtk::ConstraintTarget;

}

impl From<&model::Image> for ImageRow {
    fn from(image: &model::Image) -> Self {
        glib::Object::builder().property("image", image).build()
    }
}

impl ImageRow {
    pub(crate) fn activate(&self) {
        if let Some(image) = self.image().as_ref() {
            if image
                .image_list()
                .map(|list| list.is_selection_mode())
                .unwrap_or(false)
            {
                image.select();
            } else {
                utils::navigation_view(self).push(
                    &adw::NavigationPage::builder()
                        .title(gettext!("Image {}", utils::format_id(&image.id())))
                        .child(&view::ImageDetailsPage::from(image))
                        .build(),
                );
            }
        }
    }
}
