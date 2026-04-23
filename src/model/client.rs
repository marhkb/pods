use std::cell::OnceCell;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

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
        #[property(get = Self::image_list)]
        pub(super) image_list: OnceCell<model::ImageList>,
        #[property(get = Self::container_list)]
        pub(super) container_list: OnceCell<model::ContainerList>,
        #[property(get = Self::pod_list, nullable)]
        pub(super) pod_list: OnceCell<Option<model::PodList>>,
        #[property(get = Self::volume_list)]
        pub(super) volume_list: OnceCell<model::VolumeList>,
        #[property(get = Self::info, set, nullable)]
        pub(super) info: OnceCell<Option<model::Info>>,
        #[property(get = Self::action_list)]
        pub(super) action_list: OnceCell<model::ActionList>,
        #[property(get = Self::action_list2)]
        pub(super) action_list2: OnceCell<model::ActionList2>,
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
                        .map(Result::unwrap)
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

                    if let Some(pod_list) = obj.pod_list()
                        && let Some(pod) = container.pod_id().and_then(|id| pod_list.get_pod(&id))
                    {
                        container.set_pod(Some(&pod));
                        pod.container_list().add_container(container);
                    }

                    container
                        .mounts()
                        .iter()
                        .filter_map(|mount| {
                            obj.volume_list()
                                .get_volume(&mount.name)
                                .map(|volume| (volume, mount))
                        })
                        .for_each(|(volume, mount)| {
                            volume.container_list().add_container(container);

                            let container_volume_list = container.volume_list();
                            container_volume_list.add_volume(model::ContainerVolume::new(
                                &container_volume_list,
                                &volume,
                                mount,
                            ));
                        });
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
                        .filter_map(|container_volume| container_volume.volume())
                        .for_each(|volume| {
                            volume
                                .container_list()
                                .remove_container(container.id().as_str())
                        });
                }
            ));

            if let Some(pod_list) = obj.pod_list() {
                pod_list.connect_pod_added(clone!(
                    #[weak]
                    obj,
                    move |_, pod| {
                        obj.container_list()
                            .iter::<model::Container>()
                            .map(Result::unwrap)
                            .filter(|container| container.pod_id().as_deref() == Some(&pod.id()))
                            .for_each(|container| {
                                container.set_pod(Some(pod));
                                pod.container_list().add_container(&container);
                            });
                    }
                ));
            }

            obj.volume_list().connect_volume_added(clone!(
                #[weak]
                obj,
                move |_, volume| {
                    let container_list: Vec<_> = obj
                        .container_list()
                        .iter::<model::Container>()
                        .map(Result::unwrap)
                        .filter(|container| !container.mounts().is_empty())
                        .collect();

                    if container_list.is_empty() {
                        return;
                    }

                    volume.set_searching_containers(true);
                    let containers_left = Rc::new(AtomicUsize::new(container_list.len()));

                    container_list.iter().for_each(|container| {
                        if let Some(mount) = container
                            .mounts()
                            .iter()
                            .find(|mount| mount.name == volume.name())
                        {
                            volume.container_list().add_container(container);

                            let container_volume_list = container.volume_list();
                            container_volume_list.add_volume(model::ContainerVolume::new(
                                &container_volume_list,
                                volume,
                                mount,
                            ));
                        }

                        if containers_left.fetch_sub(1, Ordering::Relaxed) == 1 {
                            volume.set_searching_containers(false);
                        }
                    });
                }
            ));
        }
    }

    impl Client {
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

        fn pod_list(&self) -> Option<model::PodList> {
            self.pod_list
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

        fn volume_list(&self) -> model::VolumeList {
            self.volume_list
                .get_or_init(|| model::VolumeList::from(&*self.obj()))
                .to_owned()
        }

        fn info(&self) -> Option<model::Info> {
            self.info.get().cloned().flatten()
        }

        fn action_list(&self) -> model::ActionList {
            self.action_list
                .get_or_init(|| model::ActionList::from(&*self.obj()))
                .to_owned()
        }

        fn action_list2(&self) -> model::ActionList2 {
            self.action_list2
                .get_or_init(|| model::ActionList2::from(&*self.obj()))
                .to_owned()
        }
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
                    obj.image_list().refresh(err_op.clone());
                    obj.container_list().refresh(err_op.clone());
                    if let Some(pod_list) = obj.pod_list() {
                        pod_list.refresh(err_op.clone());
                    }
                    obj.volume_list().refresh(err_op.clone());

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
                            obj.container_list().handle_event(event, err_op.clone())
                        }
                        engine::dto::EventType::Image => {
                            obj.image_list().handle_event(event, err_op.clone())
                        }
                        engine::dto::EventType::Pod => {
                            if let Some(pod_list) = obj.pod_list() {
                                pod_list.handle_event(event, err_op.clone());
                            }
                        }

                        engine::dto::EventType::Volume => {
                            obj.volume_list().handle_event(event, err_op.clone())
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

                    obj.image_list().refresh(|_| {});
                    obj.container_list().refresh(|_| {});
                    if let Some(pod_list) = obj.pod_list() {
                        pod_list.refresh(|_| {});
                    }
                    obj.volume_list().refresh(|_| {});

                    log::debug!("Sleeping for {SYNC_INTERVAL} seconds until next sync");

                    glib::ControlFlow::Continue
                }
            ),
        );
    }
}
