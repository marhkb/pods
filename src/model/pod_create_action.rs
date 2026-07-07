use std::cell::OnceCell;

use adw::prelude::*;
use futures::StreamExt;
use futures::future;
use gettextrs::gettext;
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
    #[properties(wrapper_type = super::PodCreateAction)]
    pub(crate) struct PodCreateAction {
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedPodCreateOpts>,
        #[property(get)]
        pub(super) output: gtk::TextBuffer,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodCreateAction {
        const NAME: &'static str = "PodCreateAction";
        type Type = super::PodCreateAction;
        type ParentType = model::ArtifactAction;
    }

    impl ObjectImpl for PodCreateAction {
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
    pub(crate) struct PodCreateAction(ObjectSubclass<imp::PodCreateAction>)
        @extends model::Action, model::ArtifactAction;
}

impl PodCreateAction {
    pub(crate) fn new(action_list: &model::ActionList, opts: engine::opts::PodCreateOpts) -> Self {
        model::Action::builder::<Self>(action_list)
            .property("opts", model::BoxedPodCreateOpts::from(opts))
            .build()
            .exec()
    }

    fn exec(self) -> Self {
        let Some(client) = self
            .action_list()
            .and_then(|action_list| action_list.client())
        else {
            return self;
        };

        let engine = client.engine();

        let engine::opts::PodInfra::Infra {
            image, pull_latest, ..
        } = &self.opts().infra
        else {
            return self.exec_create_pod(&engine);
        };

        let Some(image) = image else {
            return self.exec_create_pod(&engine);
        };

        if !pull_latest && client.image_list().find_image(image).is_some() {
            return self.exec_create_pod(&engine);
        }

        let opts = engine::opts::ImagePullOpts {
            reference: image.to_owned(),
        };
        let abort_registration = self.setup_abort_handle();

        rt::Pipe::new(engine.images(), move |images| {
            future::Abortable::new(images.pull(opts), abort_registration).boxed()
        })
        .on_next(clone!(
            #[weak(rename_to = obj)]
            self,
            #[weak]
            engine,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move |report| match report {
                Ok(report) => match report {
                    engine::dto::ImagePullReport::Error { message } => {
                        log::warn!("error pulling infra image: {message}");
                        obj.set_failed(&message);
                        glib::ControlFlow::Break
                    }
                    engine::dto::ImagePullReport::Streaming { line } => {
                        obj.insert(&line);
                        glib::ControlFlow::Continue
                    }
                    engine::dto::ImagePullReport::Finished { .. } => {
                        obj.exec_create_pod(&engine);
                        glib::ControlFlow::Break
                    }
                },
                Err(e) => {
                    log::warn!("error pulling infra image: {e}");
                    obj.set_failed(&e.to_string());
                    glib::ControlFlow::Break
                }
            }
        ));

        self
    }

    fn exec_create_pod(self, engine: &model::Engine) -> Self {
        let engine = (**engine).clone();
        let opts = (*self.opts()).clone();
        let abort_registration = self.setup_abort_handle();

        rt::Promise::new(async move {
            future::Abortable::new(engine.pods().create(opts), abort_registration).await
        })
        .defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |pod_id| if let Ok(pod_id) = pod_id {
                match pod_id {
                    Ok(pod_id) => obj.finish(
                        pod_id,
                        clone!(
                            #[weak]
                            obj,
                            move |pod| {
                                obj.insert_line(&gettext("Pod Created"));
                                obj.set_artifact(Some(pod.upcast_ref()));
                                obj.set_state(model::ActionState::Finished);
                            }
                        ),
                    ),
                    Err(e) => {
                        log::error!("error on creating pod: {e}");
                        obj.set_failed(&e.to_string());
                    }
                }
            }
        ));

        self
    }

    fn finish<F>(&self, pod_id: String, op: F)
    where
        F: Fn(&model::Pod) + 'static,
    {
        let Some(pod_list) = self
            .action_list()
            .and_then(|action_list| action_list.client())
            .and_then(|client| client.pod_list())
        else {
            return;
        };

        match pod_list.get_pod(&pod_id) {
            Some(pod) => op(&pod),
            None => {
                pod_list.connect_pod_added(move |_, pod| {
                    if pod.id() == pod_id {
                        op(pod);
                    }
                });
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
