use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::BinExt;
use futures::StreamExt;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use sourceview5::traits::BufferExt;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image/building-page.ui")]
    pub(crate) struct BuildingPage {
        pub(super) client: WeakRef<model::Client>,
        pub(super) last_stream: RefCell<Option<String>>,
        pub(super) image: WeakRef<model::Image>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) frame_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) source_buffer: TemplateChild<sourceview5::Buffer>,
        #[template_child]
        pub(super) image_page_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BuildingPage {
        const NAME: &'static str = "PdsImageBuildingPage";
        type Type = super::BuildingPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("image.view", None, move |widget, _, _| {
                widget.view_image();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BuildingPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "client",
                    "Client",
                    "The client of this building page",
                    model::Client::static_type(),
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

            obj.action_set_enabled("image.view", false);

            let adw_style_manager = adw::StyleManager::default();
            obj.on_notify_dark(&adw_style_manager);
            adw_style_manager.connect_dark_notify(clone!(@weak obj => move |style_manager| {
                obj.on_notify_dark(style_manager);
            }));
        }
        fn dispose(&self, obj: &Self::Type) {
            let mut next = obj.first_child();
            while let Some(child) = next {
                next = child.next_sibling();
                child.unparent();
            }
        }
    }

    impl WidgetImpl for BuildingPage {}
}

glib::wrapper! {
    pub(crate) struct BuildingPage(ObjectSubclass<imp::BuildingPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<Option<&model::Client>> for BuildingPage {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::new(&[("client", &client)]).expect("Failed to build PdsImageBuildingPage")
    }
}

impl BuildingPage {
    fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    pub(crate) fn build<F>(&self, opts: podman::opts::ImageBuildOpts, op: F)
    where
        F: FnOnce(podman::Error) + Clone + 'static,
    {
        utils::run_stream_with_finish_handler(
            self.client().unwrap().podman().images(),
            move |images| match images.build(&opts) {
                Ok(stream) => stream.boxed(),
                Err(e) => {
                    log::error!("Error on building image: {e}");
                    futures::stream::empty().boxed()
                }
            },
            clone!(
                @weak self as obj => @default-return glib::Continue(false),
                move |result: podman::Result<podman::models::ImageBuildLibpod200Response>|
            {
                let imp = obj.imp();

                imp.frame_stack.set_visible_child_name("text");

                glib::Continue(match result {
                    Ok(stream) => {
                        let source_buffer = &*imp.source_buffer;
                        source_buffer.insert(&mut source_buffer.start_iter(), &stream.stream);
                        imp.last_stream.replace(Some(stream.stream));
                        true
                    }
                    Err(e) => {
                        log::error!("Error on building image: {e}");
                        op.clone()(e);
                        false
                    },
                })
            }),
            clone!(@weak self as obj => @default-return glib::Continue(false), move |_| {
                // Go To Image
                let imp = obj.imp();

                if let Some(image_id) =
                    imp.last_stream.replace(None).map(|id| id.trim().to_owned())
                {
                    let client = imp.client.upgrade().unwrap();
                    match client.image_list().get_image(&image_id) {
                        Some(image) => obj.set_image(&image),
                        None => {
                            client
                                .image_list()
                                .connect_image_added(clone!(@weak obj => move |_, image| {
                                    if image.id() == image_id {
                                        obj.set_image(image);
                                    }
                                }));
                        }
                    }
                }

                glib::Continue(false)
            }),
        );
    }

    fn view_image(&self) {
        let imp = self.imp();

        if let Some(image) = imp.image.upgrade() {
            imp.image_page_bin
                .set_child(Some(&view::ImageDetailsPage::from(&image)));
            imp.main_stack.set_visible_child(&*imp.image_page_bin);
        }
    }

    fn set_image(&self, image: &model::Image) {
        self.imp().image.set(Some(image));
        self.action_set_enabled("image.view", true);
    }

    fn on_notify_dark(&self, style_manager: &adw::StyleManager) {
        self.imp().source_buffer.set_style_scheme(
            sourceview5::StyleSchemeManager::default()
                .scheme(if style_manager.is_dark() {
                    "Adwaita-dark"
                } else {
                    "Adwaita"
                })
                .as_ref(),
        );
    }
}
