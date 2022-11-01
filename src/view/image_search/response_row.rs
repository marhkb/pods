use gtk::glib;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image-search/response-row.ui")]
    pub(crate) struct ResponseRow {
        pub(super) image_search_response: glib::WeakRef<model::ImageSearchResponse>,
        #[template_child]
        pub(super) description_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ResponseRow {
        const NAME: &'static str = "PdsImageSearchResponseRow";
        type Type = super::ResponseRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ResponseRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::ImageSearchResponse>(
                        "image-search-response",
                    )
                    .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "image-search-response" => {
                    self.image_search_response.set(value.get().unwrap());
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image-search-response" => self.obj().image_search_response().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            Self::Type::this_expression("image-search-response")
                .chain_property::<model::ImageSearchResponse>("description")
                .chain_closure::<bool>(closure!(|_: Self::Type, description: Option<&str>| {
                    !description.map(str::is_empty).unwrap_or(true)
                }))
                .bind(&*self.description_label, "visible", Some(obj));
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for ResponseRow {}
}

glib::wrapper! {
    pub(crate) struct ResponseRow(ObjectSubclass<imp::ResponseRow>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ResponseRow {
    pub(crate) fn image_search_response(&self) -> Option<model::ImageSearchResponse> {
        self.imp().image_search_response.upgrade()
    }
}
