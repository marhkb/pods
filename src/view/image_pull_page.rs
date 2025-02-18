use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImagePullPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_pull_page.ui")]
    pub(crate) struct ImagePullPage {
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePullPage {
        const NAME: &'static str = "PdsImagePullPage";
        type Type = super::ImagePullPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagePullPage {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ImagePullPage {}

    #[gtk::template_callbacks]
    impl ImagePullPage {
        #[template_callback]
        fn on_image_selected(&self, image: &str) {
            let opts = podman::opts::PullOpts::builder()
                .reference(image)
                .quiet(false)
                .build();

            let page = view::ActionPage::from(
                &self
                    .obj()
                    .client()
                    .unwrap()
                    .action_list()
                    .download_image(image, opts),
            );

            self.navigation_view.push(
                &adw::NavigationPage::builder()
                    .can_pop(false)
                    .child(&page)
                    .build(),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImagePullPage(ObjectSubclass<imp::ImagePullPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl utils::MaybeDefaultWidget for ImagePullPage {
    type Default = gtk::Widget;
}

impl From<&model::Client> for ImagePullPage {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}
