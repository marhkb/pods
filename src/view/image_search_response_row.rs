use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageSearchResponseRow)]
    #[template(file = "image_search_response_row.ui")]
    pub(crate) struct ImageSearchResponseRow {
        #[property(get, set, construct, nullable)]
        pub(super) image_search_response: glib::WeakRef<model::ImageSearchResponse>,
        #[template_child]
        pub(super) name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) description_label: TemplateChild<gtk::Label>,
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
            let description_expr =
                response_expr.chain_property::<model::ImageSearchResponse>("description");

            response_expr
                .chain_property::<model::ImageSearchResponse>("name")
                .bind(&self.name_label.get(), "label", Some(obj));

            description_expr.bind(&self.description_label.get(), "label", Some(obj));

            description_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, description: Option<&str>| {
                    !description.map(str::is_empty).unwrap_or(true)
                }))
                .bind(&*self.description_label, "visible", Some(obj));

            response_expr
                .chain_property::<model::ImageSearchResponse>("stars")
                .bind(&self.stars_label.get(), "label", Some(obj));
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

impl From<&model::ImageSearchResponse> for ImageSearchResponseRow {
    fn from(response: &model::ImageSearchResponse) -> Self {
        glib::Object::builder()
            .property("image-search-response", response)
            .build()
    }
}
