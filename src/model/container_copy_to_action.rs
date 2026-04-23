use std::cell::Cell;
use std::cell::OnceCell;
use std::ffi::OsStr;
use std::path::PathBuf;

use adw::prelude::*;
use futures::future;
use futures::stream;
use glib::Properties;
use glib::clone;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::model;
use crate::model::prelude::*;
use crate::rt;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, glib::Enum)]
#[enum_type(name = "ContainerCopyToActionOngoingState")]
pub(crate) enum ContainerCopyToActionOngoingState {
    #[default]
    CreateTar = 0,
    UnwrapTar = 1,
    CopyBytes = 2,
}

mod imp {
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ContainerCopyToAction)]
    pub(crate) struct ContainerCopyToAction {
        #[property(get, set, construct_only)]
        pub(super) container: glib::WeakRef<model::Container>,

        #[property(get, set, construct_only)]
        pub(super) directory: OnceCell<bool>,
        #[property(get, set, construct_only)]
        pub(super) host_path: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) container_path: OnceCell<String>,

        #[property(get, set, default)]
        pub(super) ongoing_state: Cell<ContainerCopyToActionOngoingState>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCopyToAction {
        const NAME: &'static str = "ContainerCopyToAction";
        type Type = super::ContainerCopyToAction;
        type ParentType = model::Action;
    }

    impl ObjectImpl for ContainerCopyToAction {
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
    pub(crate) struct ContainerCopyToAction(ObjectSubclass<imp::ContainerCopyToAction>)
        @extends model::Action, model::ArtifactAction;
}

impl ContainerCopyToAction {
    pub(crate) fn new(
        action_list: &model::ActionList,
        container: &model::Container,
        directory: bool,
        host_path: &str,
        container_path: &str,
    ) -> Self {
        model::Action::builder::<Self>(action_list)
            .property("container", container)
            .property("directory", directory)
            .property("host-path", host_path)
            .property("container-path", container_path)
            .build()
            .exec()
    }

    fn exec(self) -> Self {
        let abort_registration = self.setup_abort_handle();

        rt::Promise::new({
            let host_path = self.host_path();
            let directory = self.directory();

            async move {
                let mut ar = tokio_tar::Builder::new(Vec::new());

                future::Abortable::new(
                    async move {
                        let host_path = PathBuf::from(host_path);
                        let file_name = host_path.file_name();
                        if directory {
                            ar.append_dir_all(
                                file_name.unwrap_or_else(|| OsStr::new(".")),
                                &host_path,
                            )
                            .await
                        } else {
                            match tokio::fs::File::open(&host_path).await {
                                Ok(mut file) => ar.append_file(file_name.unwrap(), &mut file).await,
                                Err(e) => Err(e),
                            }
                        }
                        .map(|_| ar)
                    },
                    abort_registration,
                )
                .await
            }
        })
        .defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |result| if let Ok(result) = result {
                match result {
                    Ok(ar) => {
                        obj.set_ongoing_state(ContainerCopyToActionOngoingState::UnwrapTar);

                        let abort_registration = obj.setup_abort_handle();
                        rt::Promise::new(future::Abortable::new(
                            ar.into_inner(),
                            abort_registration,
                        ))
                        .defer(clone!(
                            #[weak]
                            obj,
                            move |result| if let Ok(result) = result {
                                match result {
                                    Ok(buf) => {
                                        obj.set_ongoing_state(
                                            ContainerCopyToActionOngoingState::CopyBytes,
                                        );

                                        let abort_registration = obj.setup_abort_handle();

                                        let Some(api) =
                                            obj.container().and_then(|container| container.api())
                                        else {
                                            return;
                                        };

                                        rt::Promise::new({
                                            let container_path = obj.container_path();
                                            async move {
                                                stream::Abortable::new(
                                                    api.copy_to(container_path, buf),
                                                    abort_registration,
                                                )
                                                .await
                                            }
                                        })
                                        .defer(clone!(
                                            #[weak]
                                            obj,
                                            move |result| if result.is_ok() {
                                                obj.set_state(model::ActionState::Finished);
                                            }
                                        ));
                                    }
                                    Err(e) => obj.set_failed(&e.to_string()),
                                }
                            }
                        ));
                    }
                    Err(e) => obj.set_failed(&e.to_string()),
                }
            }
        ));

        self
    }
}
