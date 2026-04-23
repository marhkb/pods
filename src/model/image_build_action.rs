use std::cell::OnceCell;

use adw::prelude::*;
use futures::StreamExt;
use futures::stream;
use glib::Properties;
use glib::clone;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;
use crate::model;
use crate::model::prelude::*;
use crate::rt;

mod imp {
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ImageBuildAction)]
    pub(crate) struct ImageBuildAction {
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedImageBuildOpts>,
        #[property(get)]
        pub(super) output: gtk::TextBuffer,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageBuildAction {
        const NAME: &'static str = "ImageBuildAction";
        type Type = super::ImageBuildAction;
        type ParentType = model::ArtifactAction;
    }

    impl ObjectImpl for ImageBuildAction {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageBuildAction(ObjectSubclass<imp::ImageBuildAction>)
        @extends model::BaseAction, model::ArtifactAction;
}

impl ImageBuildAction {
    pub(crate) fn new(
        action_list: &model::ActionList2,
        opts: engine::opts::ImageBuildOpts,
    ) -> Self {
        model::BaseAction::builder::<Self>(action_list)
            .property("opts", model::BoxedImageBuildOpts::from(opts))
            .build()
            .exec()
    }

    fn exec(self) -> Self {
        let Some(engine) = self
            .action_list()
            .and_then(|action_list| action_list.client())
            .map(|client| client.engine())
        else {
            return self;
        };

        let opts = (*self.opts()).clone();
        let abort_registration = self.setup_abort_handle();

        rt::Pipe::new(engine.images(), move |images| match images.build(opts) {
            Ok(stream) => stream::Abortable::new(stream, abort_registration).boxed(),
            Err(e) => {
                log::error!("Error on building image: {e}");
                futures::stream::empty().boxed()
            }
        })
        .on_next(clone!(
            #[weak(rename_to = obj)]
            self,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move |report| match report {
                Ok(report) => match report {
                    engine::dto::ImageBuildReport::Error { message } => {
                        log::warn!("error building image: {message}");
                        obj.set_failed(&message);
                        glib::ControlFlow::Break
                    }
                    engine::dto::ImageBuildReport::Streaming { line } => {
                        obj.insert(&line);
                        glib::ControlFlow::Continue
                    }
                    engine::dto::ImageBuildReport::Finished { image_id } => {
                        obj.finish(image_id);
                        obj.set_state(model::ActionState2::Finished);
                        glib::ControlFlow::Break
                    }
                },
                Err(e) => {
                    log::warn!("error building image: {e}");
                    obj.set_failed(&e.to_string());
                    glib::ControlFlow::Break
                }
            }
        ));

        self
    }

    fn finish(&self, image_id: String) {
        let Some(image_list) = self
            .action_list()
            .and_then(|action_list| action_list.client())
            .map(|client| client.image_list())
        else {
            return;
        };

        match image_list.get_image(&image_id) {
            Some(image) => {
                self.set_artifact(Some(image.upcast_ref()));
                self.set_state(model::ActionState2::Finished);
            }
            None => {
                image_list.connect_image_added(clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_, image| {
                        if image.id() == image_id {
                            obj.set_artifact(Some(image.upcast_ref()));
                            obj.set_state(model::ActionState2::Finished);
                        }
                    }
                ));
            }
        }
    }

    fn insert(&self, text: &str) {
        let output = self.output();
        let mut iter = output.end_iter();

        output.insert(&mut iter, text);
    }

    fn insert_line(&self, text: &str) {
        self.insert(text);
        self.insert("\n");
    }
}
