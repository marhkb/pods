use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageSelectionRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_selection_row.ui")]
    pub(crate) struct ImageSelectionRow {
        #[property(get, set, nullable)]
        pub(super) image: glib::WeakRef<model::Image>,
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageSelectionRow {
        const NAME: &'static str = "PdsImageSelectionRow";
        type Type = super::ImageSelectionRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageSelectionRow {
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
            let image_repo_tags_expr = image_expr.chain_property::<model::Image>("repo-tags");
            let image_first_repo_tag_or_id_expr = gtk::ClosureExpression::new::<String>(
                [image_expr, image_repo_tags_expr],
                closure!(
                    |_: Self::Type, image: &model::Image, repo_tags: &model::RepoTagList| {
                        repo_tags
                            .get(0)
                            .map(|repo_tag| repo_tag.full())
                            .unwrap_or_else(|| utils::format_id(&image.id()).to_owned())
                    }
                ),
            );

            image_first_repo_tag_or_id_expr.bind(&*self.label, "label", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ImageSelectionRow {}
}

glib::wrapper! {
    pub(crate) struct ImageSelectionRow(ObjectSubclass<imp::ImageSelectionRow>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
