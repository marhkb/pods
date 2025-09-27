use std::cell::OnceCell;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use futures::StreamExt;
use gio::prelude::*;
use glib::Properties;
use glib::clone;
use glib::subclass::prelude::*;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::model::AbstractContainerListExt;
use crate::monad_boxed_type;
use crate::podman;
use crate::rt;

/// Sync interval in seconds
const SYNC_INTERVAL: u32 = 15;

monad_boxed_type!(pub(crate) BoxedPodman(podman::Podman) impls Debug);

#[derive(Clone, Debug)]
pub(crate) enum ClientError {
    Images,
    Containers,
    Pods,
    Volumes,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Client)]
    pub(crate) struct Client {
        #[property(get, set, construct_only)]
        pub(super) connection: OnceCell<model::Connection>,
        #[property(get, set, construct_only)]
        pub(super) podman: OnceCell<BoxedPodman>,
        #[property(get = Self::version, nullable)]
        pub(super) version: OnceCell<Option<String>>,
        #[property(get = Self::cpus, nullable)]
        pub(super) cpus: OnceCell<i64>,
        #[property(get = Self::image_list)]
        pub(super) image_list: OnceCell<model::ImageList>,
        #[property(get = Self::container_list)]
        pub(super) container_list: OnceCell<model::ContainerList>,
        #[property(get = Self::pod_list)]
        pub(super) pod_list: OnceCell<model::PodList>,
        #[property(get = Self::volume_list)]
        pub(super) volume_list: OnceCell<model::VolumeList>,
        #[property(get = Self::action_list)]
        pub(super) action_list: OnceCell<model::ActionList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Client {
        const NAME: &'static str = "Client";
        type Type = super::Client;
    }

    impl ObjectImpl for Client {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            obj.image_list().connect_image_added(clone!(
                #[weak]
                obj,
                move |_, image| {
                    obj.container_list()
                        .iter::<model::Container>()
                        .map(|container| container.unwrap())
                        .filter(|container| container.image_id() == image.id())
                        .for_each(|container| {
                            container.set_image(Some(image));
                            image.container_list().add_container(&container);
                        });
                }
            ));

            obj.container_list().connect_container_added(clone!(
                #[weak]
                obj,
                move |_, container| {
                    let image = obj.image_list().get_image(container.image_id().as_str());
                    if let Some(ref image) = image {
                        container.set_image(Some(image));
                        image.container_list().add_container(container);
                    }

                    if let Some(pod) = container
                        .pod_id()
                        .and_then(|id| obj.pod_list().get_pod(&id))
                    {
                        container.set_pod(Some(&pod));
                        pod.container_list().add_container(container);
                    }

                    if !container.mounts().is_empty() {
                        container.inspect(clone!(
                            #[weak]
                            obj,
                            move |result| {
                                if let Ok(container) = result {
                                    container
                                        .data()
                                        .unwrap()
                                        .mounts()
                                        .values()
                                        .filter_map(|mount| {
                                            obj.volume_list()
                                                .get_volume(mount.name.as_ref().unwrap())
                                                .map(|volume| (volume, mount))
                                        })
                                        .for_each(|(volume, mount)| {
                                            volume.container_list().add_container(&container);

                                            let container_volume_list = container.volume_list();
                                            container_volume_list.add_volume(
                                                model::ContainerVolume::new(
                                                    &container_volume_list,
                                                    &volume,
                                                    mount.clone(),
                                                ),
                                            );
                                        });
                                }
                            }
                        ));
                    }
                }
            ));
            obj.container_list().connect_container_removed(clone!(
                #[weak]
                obj,
                move |_, container| {
                    if let Some(image) = obj.image_list().get_image(container.image_id().as_str()) {
                        image
                            .container_list()
                            .remove_container(container.id().as_str());
                    }

                    if let Some(pod) = container.pod() {
                        pod.container_list()
                            .remove_container(container.id().as_str());
                    }

                    container
                        .volume_list()
                        .iter::<model::ContainerVolume>()
                        .map(|result| result.unwrap())
                        .for_each(|container_volume| {
                            if let Some(volume) = container_volume.volume() {
                                volume
                                    .container_list()
                                    .remove_container(container.id().as_str());
                            }
                        });
                }
            ));

            obj.pod_list().connect_pod_added(clone!(
                #[weak]
                obj,
                move |_, pod| {
                    obj.container_list()
                        .iter::<model::Container>()
                        .map(|container| container.unwrap())
                        .filter(|container| container.pod_id().as_deref() == Some(&pod.id()))
                        .for_each(|container| {
                            container.set_pod(Some(pod));
                            pod.container_list().add_container(&container);
                        });
                }
            ));

            obj.volume_list().connect_volume_added(clone!(
                #[weak]
                obj,
                move |_, volume| {
                    let container_list: Vec<_> = obj
                        .container_list()
                        .iter::<model::Container>()
                        .map(|container| container.unwrap())
                        .filter(|container| !container.mounts().is_empty())
                        .collect();

                    if container_list.is_empty() {
                        return;
                    }

                    volume.set_searching_containers(true);
                    let containers_left = Rc::new(AtomicUsize::new(container_list.len()));

                    container_list.iter().for_each(|container| {
                        container.inspect(clone!(
                            #[weak]
                            volume,
                            #[strong]
                            containers_left,
                            move |result| {
                                if let Ok(container) = result
                                    && let Some(mount) =
                                        container.data().unwrap().mounts().values().find(|mount| {
                                            mount.name.as_ref() == Some(&volume.inner().name)
                                        })
                                {
                                    volume.container_list().add_container(&container);

                                    let container_volume_list = container.volume_list();
                                    container_volume_list.add_volume(model::ContainerVolume::new(
                                        &container_volume_list,
                                        &volume,
                                        mount.clone(),
                                    ));
                                }

                                if containers_left.fetch_sub(1, Ordering::Relaxed) == 1 {
                                    volume.set_searching_containers(false);
                                }
                            }
                        ));
                    });
                }
            ));
        }
    }

    impl Client {
        fn version(&self) -> Option<String> {
            self.version.get().cloned().flatten()
        }

        fn cpus(&self) -> i64 {
            self.cpus.get().cloned().unwrap_or(-1)
        }

        fn image_list(&self) -> model::ImageList {
            self.image_list
                .get_or_init(|| model::ImageList::from(&*self.obj()))
                .to_owned()
        }

        fn container_list(&self) -> model::ContainerList {
            self.container_list
                .get_or_init(|| model::ContainerList::from(&*self.obj()))
                .to_owned()
        }

        fn pod_list(&self) -> model::PodList {
            self.pod_list
                .get_or_init(|| model::PodList::from(&*self.obj()))
                .to_owned()
        }

        fn volume_list(&self) -> model::VolumeList {
            self.volume_list
                .get_or_init(|| model::VolumeList::from(&*self.obj()))
                .to_owned()
        }

        fn action_list(&self) -> model::ActionList {
            self.action_list
                .get_or_init(|| model::ActionList::from(&*self.obj()))
                .to_owned()
        }
    }
}

glib::wrapper! {
    pub(crate) struct Client(ObjectSubclass<imp::Client>);
}

impl TryFrom<&model::Connection> for Client {
    type Error = podman::Error;

    fn try_from(connection: &model::Connection) -> Result<Self, Self::Error> {
        podman::Podman::new(connection.url()).map(|podman| {
            let obj: Self = glib::Object::builder()
                .property("connection", connection)
                .property("podman", BoxedPodman::from(podman.clone()))
                .build();

            rt::Promise::new(async move { podman.info().await }).defer(clone!(
                #[weak]
                obj,
                move |info| match info {
                    Ok(info) => {
                        obj.set_version(info.version.unwrap().version);
                        obj.set_cpus(info.host.unwrap().cpus);
                    }
                    Err(e) => {
                        log::error!("Error on retrieving podmnan info: {e}");

                        obj.set_version(None);
                        obj.set_cpus(None);
                    }
                }
            ));

            obj
        })
    }
}

impl Client {
    fn set_version(&self, value: Option<String>) {
        self.imp().version.set(value).unwrap();
        self.notify_version();
    }

    fn set_cpus(&self, value: Option<i64>) {
        self.imp().cpus.set(value.unwrap_or(-1)).unwrap();
        self.notify_cpus();
    }

    pub(crate) fn check_service<T, E, F>(&self, op: T, err_op: E, finish_op: F)
    where
        T: FnOnce() + 'static,
        E: FnOnce(ClientError) + Clone + 'static,
        F: FnOnce(podman::Error) + Clone + 'static,
    {
        rt::Promise::new({
            let podman = self.podman();
            async move { podman.ping().await }
        })
        .defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |result| match result {
                Ok(_) => {
                    obj.image_list().refresh({
                        let err_op = err_op.clone();
                        |_| err_op(ClientError::Images)
                    });
                    obj.container_list().refresh(None, {
                        let err_op = err_op.clone();
                        |_| err_op(ClientError::Containers)
                    });
                    obj.pod_list().refresh(None, {
                        let err_op = err_op.clone();
                        |_| err_op(ClientError::Pods)
                    });
                    obj.volume_list().refresh({
                        let err_op = err_op.clone();
                        |_| err_op(ClientError::Volumes)
                    });

                    op();
                    obj.start_event_listener(err_op, finish_op);
                    obj.start_refresh_interval();
                }
                Err(e) => {
                    log::error!("Could not connect to Podman: {e}");
                    // No need to show a toast. The start service page is enough.
                }
            }
        ));
    }

    fn start_event_listener<E, F>(&self, err_op: E, finish_op: F)
    where
        E: FnOnce(ClientError) + Clone + 'static,
        F: FnOnce(podman::Error) + Clone + 'static,
    {
        rt::Pipe::new(self.podman(), |podman| {
            podman
                .events(&podman::opts::EventsOpts::builder().build())
                .boxed()
        })
        .on_next(clone!(
            #[weak(rename_to = obj)]
            self,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move |result: podman::Result<podman::models::Event>| {
                match result {
                    Ok(event) => {
                        log::debug!("Event: {event:?}");
                        match event
                            // spellchecker:off
                            .typ
                            // spellchecker:on
                            .as_str()
                        {
                            // spellchecker:disable-line
                            "image" => obj.image_list().handle_event(event, {
                                let err_op = err_op.clone();
                                |_| err_op(ClientError::Images)
                            }),
                            "container" => obj.container_list().handle_event(event, {
                                let err_op = err_op.clone();
                                |_| err_op(ClientError::Containers)
                            }),
                            "pod" => obj.pod_list().handle_event(event, {
                                let err_op = err_op.clone();
                                |_| err_op(ClientError::Pods)
                            }),
                            "volume" => obj.volume_list().handle_event(event, {
                                let err_op = err_op.clone();
                                |_| err_op(ClientError::Volumes)
                            }),
                            other => log::warn!("Unhandled event type: {other}"),
                        }
                        glib::ControlFlow::Continue
                    }
                    Err(e) => {
                        log::error!("Stopping image event stream due to error: {e}");
                        finish_op.clone()(e);
                        glib::ControlFlow::Break
                    }
                }
            }
        ));
    }

    /// This is needed to keep track of images and containers that are managed by Buildah.
    /// See https://github.com/marhkb/pods/issues/306
    fn start_refresh_interval(&self) {
        glib::timeout_add_seconds_local(
            SYNC_INTERVAL,
            clone!(
                #[weak(rename_to = obj)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    log::debug!("Syncing images, containers and pods");

                    obj.image_list().refresh(|_| {});
                    obj.container_list().refresh(None, |_| {});
                    obj.pod_list().refresh(None, |_| {});

                    log::debug!("Sleeping for {SYNC_INTERVAL} until next sync");

                    glib::ControlFlow::Continue
                }
            ),
        );
    }
}
