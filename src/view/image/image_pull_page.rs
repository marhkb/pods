use adw::subclass::prelude::*;
use adw::traits::BinExt;
use gettextrs::gettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;
use crate::window::Window;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image-pull-page.ui")]
    pub(crate) struct ImagePullPage {
        pub(super) client: WeakRef<model::Client>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) image_search_widget: TemplateChild<view::ImageSearchWidget>,
        #[template_child]
        pub(super) image_pulling_page: TemplateChild<view::ImagePullingPage>,
        #[template_child]
        pub(super) image_page_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePullPage {
        const NAME: &'static str = "ImagePullPage";
        type Type = super::ImagePullPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("navigation.go-first", None, move |widget, _, _| {
                widget.navigate_to_first();
            });
            klass.install_action("navigation.back", None, move |widget, _, _| {
                widget.navigate_back();
            });

            klass.install_action("image.pull", None, |widget, _, _| {
                widget.pull();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagePullPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "client",
                    "Client",
                    "The client of this ImagePullPage",
                    model::Client::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
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
                "client" => self.client.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "client" => obj.client().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.action_set_enabled("image.pull", false);
            self.image_search_widget.connect_notify_local(
                Some("selected-image"),
                clone!(@weak obj => move |widget, _| {
                    obj.action_set_enabled("image.pull", widget.selected_image().is_some());
                }),
            );
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.stack.unparent();
        }
    }

    impl WidgetImpl for ImagePullPage {
        fn realize(&self, widget: &Self::Type) {
            self.parent_realize(widget);

            widget.action_set_enabled(
                "navigation.go-first",
                widget.previous_leaflet_overlay() != widget.root_leaflet_overlay(),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImagePullPage(ObjectSubclass<imp::ImagePullPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<Option<&model::Client>> for ImagePullPage {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::new(&[("client", &client)]).expect("Failed to create ImagePullPage")
    }
}

impl ImagePullPage {
    fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    fn pull(&self) {
        let imp = self.imp();

        if let Some(search_response) = imp.image_search_widget.selected_image() {
            let opts = podman::opts::PullOpts::builder()
                .reference(format!(
                    "{}:{}",
                    search_response.name().unwrap(),
                    imp.image_search_widget.tag(),
                ))
                .quiet(false)
                .build();

            imp.stack.set_visible_child(&*imp.image_pulling_page);

            imp.image_pulling_page.pull(
                opts,
                clone!(@weak self as obj => move |result| match result {
                    Ok(report) => {
                        let imp = obj.imp();

                        let image_id = report.id.unwrap();
                        let client = imp.client.upgrade().unwrap();
                        match client.image_list().get_image(&image_id) {
                            Some(image) => obj.switch_to_image(&image),
                            None => {
                                client.image_list().connect_image_added(
                                    clone!(@weak obj => move |_, image| {
                                        if image.id() == image_id.as_str() {
                                            obj.switch_to_image(image);
                                        }
                                    }),
                                );
                            }
                        }
                    }
                    Err(e) => obj.on_pull_error(&e.to_string())
                }),
            );
        }
    }

    fn on_pull_error(&self, msg: &str) {
        self.imp().stack.set_visible_child_name("pull-settings");
        log::error!("Failed to pull image: {}", msg);
        utils::show_error_toast(self, &gettext("Failed to pull image"), msg);
    }

    fn switch_to_image(&self, image: &model::Image) {
        let imp = self.imp();
        imp.image_page_bin
            .set_child(Some(&view::ImageDetailsPage::from(image)));
        imp.stack.set_visible_child(&*imp.image_page_bin);
    }

    fn navigate_to_first(&self) {
        self.root_leaflet_overlay().hide_details();
    }

    fn navigate_back(&self) {
        self.previous_leaflet_overlay().hide_details();
    }

    fn previous_leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::find_parent_leaflet_overlay(self)
    }

    fn root_leaflet_overlay(&self) -> view::LeafletOverlay {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .leaflet_overlay()
    }
}
