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
use std::cell::Cell;

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
        #[property(get, set)]
        pub(super) written_bytes: Cell<u64>,
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
                        obj.set_failed(&gettext("Container has been removed"));
                        return;
                    };

                    let writer = Arc::new(Mutex::new(BufWriter::new(file)));

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
                                    obj.set_written_bytes(written as u64);
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
                            rt::Promise::new({
                                let writer = writer.clone();
                                async move { writer.lock().await.flush().await }
                            })
                            .defer(clone!(
                                #[weak]
                                obj,
                                move |result| {
                                    match result {
                                        Ok(_) => obj.set_state(model::ActionState2::Finished),
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
}
