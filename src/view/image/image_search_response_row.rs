use gtk::glib;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image-search-response-row.ui")]
    pub(crate) struct ImageSearchResponseRow {
        pub(super) image_search_response: WeakRef<model::ImageSearchResponse>,
        #[template_child]
        pub(super) left_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) description_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) right_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageSearchResponseRow {
        const NAME: &'static str = "ImageSearchResponseRow";
        type Type = super::ImageSearchResponseRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageSearchResponseRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "image-search-response",
                    "Image-Search-Response",
                    "The image search response of this ImageSearchResponseRow",
                    model::ImageSearchResponse::static_type(),
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
                "image-search-response" => {
                    self.image_search_response.set(value.get().unwrap());
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image-search-response" => obj.image_search_response().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            Self::Type::this_expression("image-search-response")
                .chain_property::<model::ImageSearchResponse>("description")
                .chain_closure::<bool>(closure!(|_: glib::Object, description: Option<&str>| {
                    !description.map(str::is_empty).unwrap_or(true)
                }))
                .bind(&*self.description_label, "visible", Some(obj));
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.left_box.unparent();
            self.right_box.unparent();
        }
    }

    impl WidgetImpl for ImageSearchResponseRow {}
}

glib::wrapper! {
    pub(crate) struct ImageSearchResponseRow(ObjectSubclass<imp::ImageSearchResponseRow>)
        @extends gtk::Widget;
}

impl ImageSearchResponseRow {
    pub(crate) fn image_search_response(&self) -> Option<model::ImageSearchResponse> {
        self.imp().image_search_response.upgrade()
    }
}
