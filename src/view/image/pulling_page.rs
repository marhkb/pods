use adw::subclass::prelude::*;
use anyhow::anyhow;
use futures::StreamExt;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::podman;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image/pulling-page.ui")]
    pub(crate) struct PullingPage {
        pub(super) client: WeakRef<model::Client>,
        #[template_child]
        pub(super) stream_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PullingPage {
        const NAME: &'static str = "PdsImagePullingPage";
        type Type = super::PullingPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PullingPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "client",
                    "Client",
                    "The client of this image pulling page",
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

        fn dispose(&self, obj: &Self::Type) {
            let mut next = obj.first_child();
            while let Some(child) = next {
                next = child.next_sibling();
                child.unparent();
            }
        }
    }

    impl WidgetImpl for PullingPage {}
}

glib::wrapper! {
    pub(crate) struct PullingPage(ObjectSubclass<imp::PullingPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PullingPage {
    fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    pub(crate) fn pull<F>(&self, opts: podman::opts::PullOpts, op: F)
    where
        F: FnOnce(anyhow::Result<podman::models::LibpodImagesPullReport>) + Clone + 'static,
    {
        utils::run_stream(
            self.client().unwrap().podman().images(),
            move |images| images.pull(&opts).boxed(),
            clone!(
                @weak self as obj => @default-return glib::Continue(false),
                move |result: podman::Result<podman::models::LibpodImagesPullReport>|
            {
                glib::Continue(match result {
                    Ok(report) => match report.error {
                        Some(error) => {
                            op.clone()(Err(anyhow!(error)));
                            false
                        }
                        None => match report.stream {
                            Some(stream) => {
                                obj.imp().stream_label.set_label(&stream.replace('\n', ""));
                                true
                            }
                            None => {
                                op.clone()(Ok(report));
                                false
                            }
                        }
                    }
                    Err(e) => {
                        op.clone()(Err(anyhow::Error::from(e)));
                        false
                    },
                })
            }),
        );
    }
}
