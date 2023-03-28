use futures::StreamExt;
use glib::prelude::ObjectExt;
use glib::Properties;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::ListModelExtManual;
use gtk::prelude::ParamSpecBuilderExt;
use gtk::subclass::prelude::*;
use once_cell::unsync::OnceCell as UnsyncOnceCell;

use crate::model;
use crate::model::AbstractContainerListExt;
use crate::monad_boxed_type;
use crate::podman;
use crate::utils;

/// Sync interval in seconds
const SYNC_INTERVAL: u32 = 15;

monad_boxed_type!(pub(crate) BoxedPodman(podman::Podman) impls Debug);

#[derive(Clone, Debug)]
pub(crate) enum ClientError {
    Images,
    Containers,
    Pods,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Client)]
    pub(crate) struct Client {
        #[property(get, set, construct_only)]
        pub(super) connection: UnsyncOnceCell<model::Connection>,
        #[property(get, set, construct_only)]
        pub(super) podman: UnsyncOnceCell<BoxedPodman>,
        #[property(get = Self::version, nullable)]
        pub(super) version: UnsyncOnceCell<Option<String>>,
        #[property(get = Self::image_list)]
        pub(super) image_list: UnsyncOnceCell<model::ImageList>,
        #[property(get = Self::container_list)]
        pub(super) container_list: UnsyncOnceCell<model::ContainerList>,
        #[property(get = Self::pod_list)]
        pub(super) pod_list: UnsyncOnceCell<model::PodList>,
        #[property(get = Self::action_list)]
        pub(super) action_list: UnsyncOnceCell<model::ActionList>,
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

            obj.image_list()
                .connect_image_added(clone!(@weak obj => move |_, image| {
                    obj.container_list()
                        .iter::<model::Container>()
                        .map(|container| container.unwrap())
                        .filter(|container| container.image_id() == image.id())
                        .for_each(|container| {
                            container.set_image(image);
                            image.add_container(&container);
                        });
                }));

            obj.container_list()
                .connect_container_added(clone!(@weak obj => move |_, container| {
                    let image = obj.image_list().get_image(container.image_id().as_str());
                    if let Some(ref image) = image {
                        container.set_image(image);
                        image.add_container(container);
                    }

                    if let Some(pod) = container.pod_id().and_then(|id| obj.pod_list().get_pod(&id))
                    {
                        container.set_pod(&pod);
                        pod.container_list().add_container(container);
                    }
                }));
            obj.container_list().connect_container_removed(
                clone!(@weak obj => move |_, container| {
                    if let Some(image) = obj.image_list().get_image(container.image_id().as_str()) {
                        image.remove_container(container.id().as_str());
                    }

                    if let Some(pod) = container.pod() {
                        pod.container_list().remove_container(container.id().as_str());
                    }
                }),
            );

            obj.pod_list()
                .connect_pod_added(clone!(@weak obj => move |_, pod| {
                    obj.container_list()
                        .iter::<model::Container>()
                        .map(|container| container.unwrap())
                        .filter(|container| container.pod_id().as_deref() == Some(&pod.id()))
                        .for_each(|container| {
                            container.set_pod(pod);
                            pod.container_list().add_container(&container);
                        });
                }));
        }
    }

    impl Client {
        fn version(&self) -> Option<String> {
            self.version.get().cloned().flatten()
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

            utils::do_async(
                async move {
                    podman
                        .version()
                        .await
                        .ok()
                        .and_then(|version| version.version)
                },
                clone!(@weak obj => move |version| {
                    obj.set_version(version);
                }),
            );

            obj
        })
    }
}

impl Client {
    fn set_version(&self, value: Option<String>) {
        self.imp().version.set(value).unwrap();
        self.notify("version");
    }

    pub(crate) fn check_service<T, E, F>(&self, op: T, err_op: E, finish_op: F)
    where
        T: FnOnce() + 'static,
        E: FnOnce(ClientError) + Clone + 'static,
        F: FnOnce(podman::Error) + Clone + 'static,
    {
        utils::do_async(
            {
                let podman = self.podman();
                async move { podman.ping().await }
            },
            clone!(@weak self as obj => move |result| match result {
                Ok(_) => {
                    obj.image_list().refresh({
                        let err_op = err_op.clone();
                        |_| err_op(ClientError::Images)
                    });
                    obj.container_list().refresh(
                        None,
                        {
                            let err_op = err_op.clone();
                            |_| err_op(ClientError::Containers)
                        }
                    );
                    obj.pod_list().refresh(
                        None,
                        {
                            let err_op = err_op.clone();
                            |_| err_op(ClientError::Pods)
                        }
                    );

                    op();
                    obj.start_event_listener(err_op, finish_op);
                    obj.start_refresh_interval();
                }
                Err(e) => {
                    log::error!("Could not connect to Podman: {e}");
                    // No need to show a toast. The start service page is enough.
                }
            }),
        );
    }

    fn start_event_listener<E, F>(&self, err_op: E, finish_op: F)
    where
        E: FnOnce(ClientError) + Clone + 'static,
        F: FnOnce(podman::Error) + Clone + 'static,
    {
        utils::run_stream(
            self.podman(),
            |podman| {
                podman
                    .events(&podman::opts::EventsOpts::builder().build())
                    .boxed()
            },
            clone!(
                @weak self as obj => @default-return glib::Continue(false),
                move |result: podman::Result<podman::models::Event>|
            {
                glib::Continue(match result {
                    Ok(event) => {
                        log::debug!("Event: {event:?}");
                        match event.typ.as_str() {
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
                            other => log::warn!("Unhandled event type: {other}"),
                        }
                        true
                    }
                    Err(e) => {
                        log::error!("Stopping image event stream due to error: {e}");
                        finish_op.clone()(e);
                        false
                    }
                })
            }),
        );
    }

    /// This is needed to keep track of images and containers that are managed by Buildah.
    /// See https://github.com/marhkb/pods/issues/306
    fn start_refresh_interval(&self) {
        glib::timeout_add_seconds_local(
            SYNC_INTERVAL,
            clone!(@weak self as obj => @default-return glib::Continue(false), move || {
                log::debug!("Syncing images, containers and pods");

                obj.image_list().refresh(|_| {});
                obj.container_list().refresh(None, |_| {});
                obj.pod_list().refresh(None, |_| {});

                log::debug!("Sleeping for {SYNC_INTERVAL} until next sync");

                glib::Continue(true)
            }),
        );
    }
}
