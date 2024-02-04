use std::cell::OnceCell;

use gio::prelude::*;
use glib::Properties;
use glib::clone;
use glib::subclass::prelude::*;
use gtk::gio;
use gtk::glib;

use crate::engine;
use crate::model;
use crate::model::AbstractContainerListExt;
use crate::rt;

/// Sync interval in seconds
const SYNC_INTERVAL: u32 = 15;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Client)]
    pub(crate) struct Client {
        #[property(get, set, construct_only)]
        pub(super) connection: OnceCell<model::Connection>,
        #[property(get, set, construct_only)]
        pub(super) engine: OnceCell<model::Engine>,
        #[property(get = Self::images)]
        pub(super) images: OnceCell<model::ImageList>,
        #[property(get = Self::containers)]
        pub(super) containers: OnceCell<model::ContainerList>,
        #[property(get = Self::pods, nullable)]
        pub(super) pods: OnceCell<Option<model::PodList>>,
        #[property(get = Self::network_list)]
        pub(super) networks: OnceCell<model::NetworkList>,
        #[property(get = Self::volumes)]
        pub(super) volumes: OnceCell<model::VolumeList>,
        #[property(get = Self::info, set, nullable)]
        pub(super) info: OnceCell<Option<model::Info>>,
        #[property(get = Self::actions)]
        pub(super) actions: OnceCell<model::ActionList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Client {
        const NAME: &'static str = "PdsClient";
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

            obj.images().connect_image_added(clone!(
                #[weak]
                obj,
                move |_, image| {
                    obj.containers()
                        .iter::<model::Container>()
                        .map(Result::unwrap)
                        .filter(|container| container.image_id() == image.id())
                        .for_each(|container| {
                            link_container_image(&container, image);
                        });
                }
            ));

            obj.containers().connect_container_added(clone!(
                #[weak]
                obj,
                move |_, container| {
                    let image = obj.images().get_image(container.image_id().as_str());
                    if let Some(ref image) = image {
                        link_container_image(container, image);
                    }

                    if let Some(pods) = obj.pods()
                        && let Some(pod) = container.pod_id().and_then(|id| pods.get_pod(&id))
                    {
                        link_container_pod(container, &pod);
                        if obj.containers().initialized() {
                            pod.inspect_and_update(|e| log::error!("inspect pod: {e}"));
                        }
                    }

                    container
                        .volumes()
                        .mounts()
                        .iter()
                        .filter_map(|volume_mount| {
                            obj.volumes()
                                .get_volume(&volume_mount.name)
                                .map(|volume| (volume, volume_mount))
                        })
                        .for_each(|(volume, volume_mount)| {
                            link_container_volume(container, &volume, &volume_mount.params);
                        });
                }
            ));

            obj.containers()
                .connect_container_removed(move |_, container| {
                    if let Some(image) = container.image() {
                        image.containers().remove_container(container.id().as_str());
                    }

                    if let Some(pod) = container.pod() {
                        pod.containers().remove_container(container.id().as_str());
                    }

                    container
                        .volumes()
                        .iter::<model::ContainerVolume>()
                        .map(Result::unwrap)
                        .filter_map(|container_volume| container_volume.volume())
                        .for_each(|volume| {
                            volume
                                .containers()
                                .remove_container(container.id().as_str())
                        });
                });

            if let Some(pods) = obj.pods() {
                pods.connect_pod_added(clone!(
                    #[weak]
                    obj,
                    move |_, pod| obj
                        .containers()
                        .iter::<model::Container>()
                        .map(Result::unwrap)
                        .filter(|container| container.pod_id().as_deref() == Some(&pod.id()))
                        .for_each(|container| link_container_pod(&container, pod))
                ));
            }

            obj.volumes().connect_volume_added(clone!(
                #[weak]
                obj,
                move |_, volume| obj
                    .containers()
                    .iter::<model::Container>()
                    .map(Result::unwrap)
                    .for_each(|container| {
                        if let Some(volume_mount) = container
                            .volumes()
                            .mounts()
                            .iter()
                            .find(|volume_mount| volume_mount.name == volume.name())
                        {
                            link_container_volume(&container, volume, &volume_mount.params);
                        }
                    })
            ));

            obj.networks().connect_network_added(clone!(
                #[weak]
                obj,
                move |_, network| {
                    let container_list: Vec<_> = obj
                        .containers()
                        .iter::<model::Container>()
                        .map(|container| container.unwrap())
                        .filter(|container| !container.volumes().mounts().is_empty())
                        .collect();

                    if container_list.is_empty() {
                        return;
                    }

                    //                     container_list.iter().for_each(|container| {
                    //                         container.inspect(clone!(
                    //                             #[weak]
                    //                             network,
                    //                             #[strong]
                    //                             containers_left,
                    //                             move |result| {
                    //                                 // if let Ok(container) = result {
                    //                                 //     if let Some(mount) =
                    //                                 //         container.data().unwrap().mounts().values().find(|mount| {
                    //                                 //             mount.name.as_ref() == Some(&volume.inner().name)
                    //                                 //         })
                    //                                 //     {
                    //                                 //         volume.container_list().add_container(&container);
                    //
                    //                                 //         let container_volume_list = container.volume_list();
                    //                                 //         container_volume_list.add_volume(
                    //                                 //             model::ContainerVolume::new(
                    //                                 //                 &container_volume_list,
                    //                                 //                 &volume,
                    //                                 //                 mount.clone(),
                    //                                 //             ),
                    //                                 //         );
                    //                                 //     }
                    //                                 // }
                    //
                    //                                 // if containers_left.fetch_sub(1, Ordering::Relaxed) == 1 {
                    //                                 //     volume.set_searching_containers(false);
                    //                                 // }
                    //                             }
                    //                         ));
                    //                     });
                }
            ));
        }
    }

    impl Client {
        fn images(&self) -> model::ImageList {
            self.images
                .get_or_init(|| model::ImageList::from(&*self.obj()))
                .to_owned()
        }

        fn containers(&self) -> model::ContainerList {
            self.containers
                .get_or_init(|| model::ContainerList::from(&*self.obj()))
                .to_owned()
        }

        fn pods(&self) -> Option<model::PodList> {
            self.pods
                .get_or_init(|| {
                    let obj = &*self.obj();
                    if obj.engine().capabilities().pods() {
                        Some(model::PodList::from(obj))
                    } else {
                        None
                    }
                })
                .to_owned()
        }

        fn volumes(&self) -> model::VolumeList {
            self.volumes
                .get_or_init(|| model::VolumeList::from(&*self.obj()))
                .to_owned()
        }

        fn network_list(&self) -> model::NetworkList {
            self.networks
                .get_or_init(|| model::NetworkList::from(&*self.obj()))
                .to_owned()
        }

        fn info(&self) -> Option<model::Info> {
            self.info.get().cloned().flatten()
        }

        fn actions(&self) -> model::ActionList {
            self.actions
                .get_or_init(|| model::ActionList::from(&*self.obj()))
                .to_owned()
        }
    }

    fn link_container_image(container: &model::Container, image: &model::Image) {
        container.set_image(Some(image));
        image.containers().add_container(container);
    }

    fn link_container_pod(container: &model::Container, pod: &model::Pod) {
        container.set_pod(Some(pod));
        pod.containers().add_container(container);
    }

    fn link_container_volume(
        container: &model::Container,
        volume: &model::Volume,
        params: &engine::dto::MountParams,
    ) {
        volume.containers().add_container(container);

        let container_volume_list = container.volumes();
        container_volume_list.add_volume(model::ContainerVolume::new(
            &container_volume_list,
            volume,
            params,
        ));
    }
}

glib::wrapper! {
    pub(crate) struct Client(ObjectSubclass<imp::Client>);
}

impl Client {
    pub(crate) fn new(connection: &model::Connection, engine: engine::Engine) -> Self {
        let obj: Self = glib::Object::builder()
            .property("connection", connection)
            .property("engine", model::Engine::from(engine.clone()))
            .build();

        rt::Promise::new(async move { engine.info().await }).defer(clone!(
            #[weak]
            obj,
            move |info| obj.set_info(
                info.inspect_err(|e| log::error!("Error on retrieving engine info: {e}"))
                    .map(|info| model::Info::new(&obj, info))
                    .ok()
            )
        ));

        obj
    }

    pub(crate) fn check_service<T, E, F>(&self, op: T, err_op: E, finish_op: F)
    where
        T: FnOnce() + 'static,
        E: FnOnce(anyhow::Error) + Clone + 'static,
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        rt::Promise::new({
            let engine = self.engine().inner();
            async move { engine.ping().await }
        })
        .defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |result| match result {
                Ok(_) => {
                    obj.images().refresh(err_op.clone());
                    obj.containers().refresh(err_op.clone());
                    if let Some(pods) = obj.pods() {
                        pods.refresh(err_op.clone());
                    }
                    obj.volumes().refresh(err_op.clone());
                    obj.networks().refresh(err_op.clone());

                    op();
                    obj.start_event_listener(err_op, finish_op);
                    obj.start_refresh_interval();
                }
                Err(e) => {
                    log::error!("Could not connect to container engine: {e}");
                    // No need to show a toast. The start service page is enough.
                }
            }
        ));
    }

    fn start_event_listener<E, F>(&self, err_op: E, finish_op: F)
    where
        E: FnOnce(anyhow::Error) + Clone + 'static,
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        rt::Pipe::new(self.engine().inner(), |engine| engine.events()).on_next(clone!(
            #[weak(rename_to = obj)]
            self,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move |result| match result {
                Ok(event) => {
                    log::debug!("Event: {event:?}");
                    match event.type_() {
                        engine::dto::EventType::Container => {
                            obj.containers().handle_event(event, err_op.clone())
                        }
                        engine::dto::EventType::Image => {
                            obj.images().handle_event(event, err_op.clone())
                        }
                        engine::dto::EventType::Pod => {
                            if let Some(pods) = obj.pods() {
                                pods.handle_event(event, err_op.clone());
                            }
                        }
                        engine::dto::EventType::Network => {
                            obj.networks().handle_event(event, err_op.clone());
                        }
                        engine::dto::EventType::Volume => {
                            obj.volumes().handle_event(event, err_op.clone())
                        }
                        engine::dto::EventType::Other => {}
                    }
                    glib::ControlFlow::Continue
                }
                Err(e) => {
                    log::error!("Stopping image event stream due to error: {e}");
                    finish_op.clone()(e);
                    glib::ControlFlow::Break
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
                    log::debug!("Syncing images, containers, pods and volumes");

                    obj.images().refresh(|_| {});
                    obj.containers().refresh(|_| {});
                    if let Some(pods) = obj.pods() {
                        pods.refresh(|_| {});
                    }
                    obj.volumes().refresh(|_| {});

                    log::debug!("Sleeping for {SYNC_INTERVAL} seconds until next sync");

                    glib::ControlFlow::Continue
                }
            ),
        );
    }
}
