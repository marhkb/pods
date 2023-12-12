use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageSearchResponseRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_search_response_row.ui")]
    pub(crate) struct ImageSearchResponseRow {
        #[property(get, set, construct, nullable)]
        pub(super) image_search_response: glib::WeakRef<model::ImageSearchResponse>,
        #[template_child]
        pub(super) name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) description_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) stars_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) stars_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageSearchResponseRow {
        const NAME: &'static str = "PdsImageSearchResponseRow";
        type Type = super::ImageSearchResponseRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageSearchResponseRow {
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

            let response_expr = Self::Type::this_expression("image-search-response");
            let name_expr = response_expr.chain_property::<model::ImageSearchResponse>("name");
            let tag_expr = response_expr.chain_property::<model::ImageSearchResponse>("tag");
            let has_no_tag_expr =
                tag_expr.chain_closure::<bool>(closure!(|_: Self::Type, tag: Option<&str>| tag
                    .filter(|tag| !tag.is_empty())
                    .is_none()));
            let description_expr =
                response_expr.chain_property::<model::ImageSearchResponse>("description");
            let stars_expr = response_expr.chain_property::<model::ImageSearchResponse>("stars");

            let style_manager = adw::StyleManager::default();

            gtk::ClosureExpression::new::<String>(
                [
                    &name_expr,
                    &tag_expr,
                    &style_manager.property_expression("dark"),
                    &style_manager.property_expression("high-contrast"),
                ],
                closure!(|_: Self::Type,
                          name: String,
                          tag: Option<String>,
                          is_dark: bool,
                          is_hc| {
                    tag.map(|tag| {
                        let tag = format!(
                            "<span foreground=\"{}\"{}>{}</span>",
                            if is_dark { "#78aeed" } else { "#1c71d8" },
                            if is_hc { " weight=\"bold\"" } else { "" },
                            tag,
                        );

                        format!("{name} {tag}")
                    })
                    .unwrap_or(name)
                }),
            )
            .bind(&self.name_label.get(), "label", Some(obj));

            description_expr.bind(&self.description_label.get(), "label", Some(obj));

            description_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, description: Option<&str>| {
                    !description.map(str::is_empty).unwrap_or(true)
                }))
                .bind(&*self.description_label, "visible", Some(obj));

            has_no_tag_expr.bind(&self.stars_box.get(), "visible", Some(obj));
            stars_expr.bind(&self.stars_label.get(), "label", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ImageSearchResponseRow {}
}

glib::wrapper! {
    pub(crate) struct ImageSearchResponseRow(ObjectSubclass<imp::ImageSearchResponseRow>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ImageSearchResponseRow {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl From<&model::ImageSearchResponse> for ImageSearchResponseRow {
    fn from(response: &model::ImageSearchResponse) -> Self {
        glib::Object::builder()
            .property("image-search-response", response)
            .build()
    }
}
