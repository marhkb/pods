use adw::subclass::prelude::*;
use anyhow::anyhow;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::api;
use crate::utils;
use crate::view;
use crate::window::Window;
use crate::PODMAN;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image-pulling-page.ui")]
    pub(crate) struct ImagePullingPage {
        #[template_child]
        pub(super) stream_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePullingPage {
        const NAME: &'static str = "ImagePullingPage";
        type Type = super::ImagePullingPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("navigation.go-first", None, move |widget, _, _| {
                widget.navigate_to_first();
            });
            klass.install_action("navigation.back", None, move |widget, _, _| {
                widget.navigate_back();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagePullingPage {
        fn dispose(&self, obj: &Self::Type) {
            let mut next = obj.first_child();
            while let Some(child) = next {
                next = child.next_sibling();
                child.unparent();
            }
        }
    }

    impl WidgetImpl for ImagePullingPage {
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
    pub(crate) struct ImagePullingPage(ObjectSubclass<imp::ImagePullingPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ImagePullingPage {
    pub(crate) fn pull<F>(&self, opts: api::PullOpts, op: F)
    where
        F: FnOnce(anyhow::Result<api::LibpodImagesPullReport>) + Clone + 'static,
    {
        utils::run_stream(
            PODMAN.images().pull(&opts),
            clone!(@weak self as obj => @default-return glib::Continue(false), move |result| {
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
