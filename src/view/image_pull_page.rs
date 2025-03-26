use std::cell::OnceCell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
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
        #[property(get, set, construct_only)]
        pub(super) model: OnceCell<model::ImageSearch>,
        #[template_child]
        pub(super) bin: TemplateChild<adw::Bin>,
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
            let obj = &*self.obj();

            let model = obj.model();

            let Some(client) = model.client() else {
                return;
            };

            let opts = podman::opts::PullOpts::builder()
                .reference(image)
                .quiet(false)
                .build();

            let page = view::ActionPage::from(&client.action_list().download_image(&model));

            utils::Dialog::new(
                obj,
                &view::ActionPage::from(&client.action_list().download_image(&model)),
            )
            .present();
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImagePullPage(ObjectSubclass<imp::ImagePullPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ImagePullPage {
    pub(crate) fn new(client: &model::Client) -> Self {
        let obj: Self = glib::Object::builder()
            .property("model", model::ImageSearch::from(client))
            .build();

        // let t = view::ImageSearchPage::new(client, true, &gettext("_Download"), true);

        // t.connect_image_selected(clone!(
        //     #[weak]
        //     obj,
        //     move |_, image| obj.on_image_selected(&image)
        // ));

        // obj.imp().bin.set_child(Some(&t));

        obj
    }

    pub(crate) fn restore(model: &model::ImageSearch) -> Self {
        let obj: Self = glib::Object::builder().property("model", model).build();

        // obj.imp()
        //     .bin
        //     .set_child(Some(&view::ImageSearchPage::restore(
        //         model,
        //         true,
        //         &gettext("_Download"),
        //         true,
        //     )));

        obj
    }

    fn on_image_selected(&self, image: &str) {
        let model = self.model();

        let Some(client) = model.client() else {
            return;
        };

        self.activate_action("win.close", None).unwrap();

        let opts = podman::opts::PullOpts::builder()
            .reference(image)
            .quiet(false)
            .build();

        utils::Dialog::new(
            self,
            &view::ActionPage::from(&client.action_list().download_image(&model)),
        )
        .present();
    }
}
