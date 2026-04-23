use std::cell::OnceCell;
use std::sync::Arc;

use adw::prelude::*;
use futures::FutureExt;
use futures::StreamExt;
use futures::TryFutureExt;
use futures::future;
use futures::lock::Mutex;
use futures::stream;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use glib::subclass::prelude::*;
use gtk::glib;
use tokio::io::AsyncWriteExt;
use tokio::io::BufWriter;

use crate::model;
use crate::model::prelude::*;
use crate::rt;

mod imp {
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ContainerCopyFromAction)]
    pub(crate) struct ContainerCopyFromAction {
        #[property(get, set, construct_only)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[property(get, set, construct_only)]
        pub(super) container_path: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) host_path: OnceCell<String>,
        #[property(get)]
        pub(super) output: gtk::TextBuffer,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCopyFromAction {
        const NAME: &'static str = "ContainerCopyFromAction";
        type Type = super::ContainerCopyFromAction;
        type ParentType = model::BaseAction;
    }

    impl ObjectImpl for ContainerCopyFromAction {
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
    pub(crate) struct ContainerCopyFromAction(ObjectSubclass<imp::ContainerCopyFromAction>)
        @extends model::BaseAction, model::ArtifactAction;
}

impl ContainerCopyFromAction {
    pub(crate) fn new(
        action_list: &model::ActionList2,
        container: &model::Container,
        container_path: &str,
        host_path: &str,
    ) -> Self {
        model::BaseAction::builder::<Self>(action_list)
            .property("container", container)
            .property("container-path", container_path)
            .property("host-path", host_path)
            .build()
            .exec()
    }

    fn exec(self) -> Self {
        self.insert_line(&gettext("Writing to file…"));

        let abort_registration = self.setup_abort_handle();

        rt::Promise::new({
            let host_path = self.host_path();
            async move {
                tokio::fs::File::options()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&host_path)
                    .await
            }
        })
        .defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |result| match result {
                Err(e) => obj.set_failed(&e.to_string()),
                Ok(file) => {
                    let Some(api) = obj.container().and_then(|container| container.api()) else {
                        obj.set_failed(&gettext("Container is gone"));
                        return;
                    };

                    let writer = Arc::new(Mutex::new(BufWriter::new(file)));
                    obj.insert_line(&gettext!("Written: {}", glib::format_size(0)));

                    rt::Pipe::new(api, {
                        let container_path = obj.container_path();
                        let writer = writer.clone();
                        move |container| {
                            stream::Abortable::new(
                                container.copy_from(container_path),
                                abort_registration,
                            )
                            .scan(Ok((writer, 0)), |state: &mut anyhow::Result<_>, chunk| {
                                match state {
                                    Err(_) => future::ready(None).boxed(),
                                    Ok((writer, written)) => match chunk {
                                        Err(e) => future::ready(Some(Err(e))).boxed(),
                                        Ok(chunk) => {
                                            *written += chunk.len();

                                            let writer = writer.clone();
                                            let written = *written;
                                            async move {
                                                Some({
                                                    let mut writer = writer.lock().await;
                                                    writer
                                                        .write_all(&chunk)
                                                        .map_err(anyhow::Error::from)
                                                        .map_ok(|_| written)
                                                        .await
                                                })
                                            }
                                            .boxed()
                                        }
                                    },
                                }
                            })
                            .boxed()
                        }
                    })
                    .on_next(clone!(
                        #[weak]
                        obj,
                        #[upgrade_or]
                        glib::ControlFlow::Break,
                        move |result: anyhow::Result<usize>| {
                            match result {
                                Ok(written) => {
                                    obj.replace_last_line(&gettext!(
                                        "Written: {}",
                                        glib::format_size(written as u64)
                                    ));
                                    glib::ControlFlow::Continue
                                }
                                Err(e) => {
                                    obj.set_failed(&e.to_string());
                                    glib::ControlFlow::Break
                                }
                            }
                        }
                    ))
                    .on_finish(clone!(
                        #[weak]
                        obj,
                        move || {
                            obj.insert_line(&gettext("Flushing…"));
                            rt::Promise::new({
                                let writer = writer.clone();
                                async move { writer.lock().await.flush().await }
                            })
                            .defer(clone!(
                                #[weak]
                                obj,
                                move |result| {
                                    match result {
                                        Ok(_) => {
                                            obj.insert_line(&gettext("Finished"));
                                            obj.set_state(model::ActionState2::Finished);
                                        }
                                        Err(e) => obj.set_failed(&e.to_string()),
                                    }
                                }
                            ));
                        }
                    ));
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

    fn replace_last_line(&self, text: &str) {
        let output = self.output();

        let mut start_iter = output.start_iter();
        let mut end_iter = output.start_iter();
        end_iter.forward_line();

        output.delete(&mut start_iter, &mut end_iter);
        self.insert_line(text);
    }
}
