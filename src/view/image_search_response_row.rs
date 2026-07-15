use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

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
        pub(super) official_icon: TemplateChild<gtk::Image>,
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
            let official_expr =
                response_expr.chain_property::<model::ImageSearchResponse>("official");
            let stars_expr = response_expr.chain_property::<model::ImageSearchResponse>("stars");

            name_expr.bind(&self.name_label.get(), "label", Some(obj));
            official_expr.bind(&self.official_icon.get(), "visible", Some(obj));
            stars_expr.bind(&self.stars_label.get(), "label", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
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
        glib::Object::new()
    }
}

impl From<&model::ImageSearchResponse> for ImageSearchResponseRow {
    fn from(response: &model::ImageSearchResponse) -> Self {
        glib::Object::builder()
            .property("image-search-response", response)
            .build()
    }
}
