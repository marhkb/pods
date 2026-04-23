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
    #[properties(wrapper_type = super::ContainerCreateAction)]
    pub(crate) struct ContainerCreateAction {
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedContainerCreateOpts>,
        #[property(get)]
        pub(super) output: gtk::TextBuffer,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCreateAction {
        const NAME: &'static str = "ContainerCreateAction";
        type Type = super::ContainerCreateAction;
        type ParentType = model::ArtifactAction;
    }

    impl ObjectImpl for ContainerCreateAction {
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
    pub(crate) struct ContainerCreateAction(ObjectSubclass<imp::ContainerCreateAction>)
        @extends model::BaseAction, model::ArtifactAction;
}

impl ContainerCreateAction {
    pub(crate) fn new<F>(
        action_list: &model::ActionList2,
        opts: engine::opts::ContainerCreateOpts,
        run: bool,
        err_op: F,
    ) -> Self
    where
        F: Fn(anyhow::Result<()>) + Clone + 'static,
    {
        model::BaseAction::builder::<Self>(action_list)
            .property("opts", model::BoxedContainerCreateOpts::from(opts))
            .build()
            .exec(run, err_op)
    }

    fn exec<F>(self, run: bool, err_op: F) -> Self
    where
        F: Fn(anyhow::Result<()>) + Clone + 'static,
    {
        let Some(client) = self
            .action_list()
            .and_then(|action_list| action_list.client())
        else {
            return self;
        };

        let engine = client.engine();

        if client.image_list().find_image(&self.opts().image).is_some() && !self.opts().pull_latest
        {
            return self.exec_create_container(&engine, run, err_op);
        }

        let opts = engine::opts::ImagePullOpts {
            reference: self.opts().image.clone(),
            ..Default::default()
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
                        log::warn!("error pulling image: {message}");
                        obj.set_failed(&message);
                        glib::ControlFlow::Break
                    }
                    engine::dto::ImagePullReport::Streaming { line } => {
                        obj.insert(&line);
                        glib::ControlFlow::Continue
                    }
                    engine::dto::ImagePullReport::Finished { .. } => {
                        obj.exec_create_container(&engine, run, err_op.clone());
                        glib::ControlFlow::Break
                    }
                },
                Err(e) => {
                    log::warn!("error pulling image: {e}");
                    obj.set_failed(&e.to_string());
                    glib::ControlFlow::Break
                }
            }
        ));

        self
    }

    fn exec_create_container<F>(self, engine: &model::Engine, run: bool, err_op: F) -> Self
    where
        F: Fn(anyhow::Result<()>) + Clone + 'static,
    {
        let engine = (**engine).clone();
        let opts = (*self.opts()).clone();
        let abort_registration = self.setup_abort_handle();

        rt::Promise::new(async move {
            future::Abortable::new(engine.containers().create(opts), abort_registration).await
        })
        .defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |container_id| if let Ok(container_id) = container_id {
                match container_id {
                    Ok(container_id) => obj.finish(
                        container_id,
                        clone!(
                            #[weak]
                            obj,
                            move |container| {
                                obj.insert_line(&gettext("Container Created"));
                                obj.set_artifact(Some(container.upcast_ref()));
                                obj.set_state(model::ActionState2::Finished);

                                if run {
                                    container.start(err_op.clone());
                                }
                            }
                        ),
                    ),
                    Err(e) => {
                        log::error!("error on creating container: {e}");
                        obj.insert_line(&e.to_string());
                        obj.set_state(model::ActionState2::Failed);
                    }
                }
            }
        ));

        self
    }

    fn finish<F>(&self, container_id: String, op: F)
    where
        F: Fn(&model::Container) + 'static,
    {
        let Some(container_list) = self
            .action_list()
            .and_then(|action_list| action_list.client())
            .map(|client| client.container_list())
        else {
            return;
        };

        match container_list.get_container(&container_id) {
            Some(container) => op(&container),
            None => {
                container_list.connect_container_added(move |_, container| {
                    if container.id() == container_id {
                        op(container);
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
