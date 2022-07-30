use std::cell::Cell;

use futures::StreamExt;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::ObjectExt;
use gtk::prelude::StaticType;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::api;
use crate::model;
use crate::model::AbstractContainerListExt;
use crate::monad_boxed_type;
use crate::utils;
use crate::utils::ToTypedListModel;

monad_boxed_type!(pub(crate) BoxedPodman(api::Podman) impls Debug);

#[derive(Clone, Debug)]
pub(crate) struct ClientError {
    pub(crate) err: super::RefreshError,
    pub(crate) variant: ClientErrorVariant,
}

#[derive(Clone, Debug)]
pub(crate) enum ClientErrorVariant {
    Images,
    Containers,
    Pods,
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct Client {
        pub(super) podman: OnceCell<BoxedPodman>,
        pub(super) connection: OnceCell<model::Connection>,
        pub(super) image_list: OnceCell<model::ImageList>,
        pub(super) container_list: OnceCell<model::ContainerList>,
        pub(super) pod_list: OnceCell<model::PodList>,
        pub(super) pruning: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Client {
        const NAME: &'static str = "Client";
        type Type = super::Client;
    }

    impl ObjectImpl for Client {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "connection",
                        "Connection",
                        "The connection",
                        model::Connection::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecBoxed::new(
                        "podman",
                        "Podman",
                        "The podman API interface",
                        BoxedPodman::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecObject::new(
                        "image-list",
                        "Image List",
                        "The list of images",
                        model::ImageList::static_type(),
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecObject::new(
                        "container-list",
                        "Container List",
                        "The list of containers",
                        model::ContainerList::static_type(),
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecObject::new(
                        "pod-list",
                        "Pod List",
                        "The list of containers",
                        model::PodList::static_type(),
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecBoolean::new(
                        "pruning",
                        "Pruning",
                        "Whether images are currently pruned",
                        false,
                        glib::ParamFlags::READABLE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "connection" => self.connection.set(value.get().unwrap()).unwrap(),
                "podman" => self.podman.set(value.get().unwrap()).unwrap(),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "connection" => obj.connection().to_value(),
                "podman" => obj.podman().to_value(),
                "image-list" => obj.image_list().to_value(),
                "container-list" => obj.container_list().to_value(),
                "pod-list" => obj.pod_list().to_value(),
                "pruning" => obj.pruning().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.image_list()
                .connect_image_added(clone!(@weak obj => move |_, image| {
                    obj.container_list()
                        .to_owned()
                        .to_typed_list_model::<model::Container>()
                        .into_iter()
                        .filter(|container| container.image_id() == Some(image.id()))
                        .for_each(|container| {
                            container.set_image(Some(image));
                            image.add_container(container);
                        });
                }));

            obj.container_list()
                .connect_container_added(clone!(@weak obj => move |_, container| {
                    let image = obj.image_list().get_image(container.image_id().unwrap());
                    container.set_image(image.as_ref());
                    if let Some(image) = image {
                        image.add_container(container.to_owned());
                    }

                    if let Some(pod) = container.pod_id().and_then(|id| obj.pod_list().get_pod(id)) {
                        container.set_pod(Some(&pod));
                        pod.container_list().add_container(container.to_owned());
                    }
                }));
            obj.container_list().connect_container_removed(
                clone!(@weak obj => move |_, container| {
                    if let Some(image) = container
                        .image_id()
                        .and_then(|id| obj.image_list().get_image(id))
                    {
                        image.remove_container(container.id().unwrap());
                    }
                }),
            );

            obj.pod_list()
                .connect_pod_added(clone!(@weak obj => move |_, pod| {
                    obj.container_list()
                        .to_owned()
                        .to_typed_list_model::<model::Container>()
                        .into_iter()
                        .filter(|container| container.pod_id() == Some(pod.id()))
                        .for_each(|container| {
                            container.set_pod(Some(pod));
                            pod.container_list().add_container(container);
                        });
                }));
        }
    }
}

glib::wrapper! {
    pub(crate) struct Client(ObjectSubclass<imp::Client>);
}

impl TryFrom<&model::Connection> for Client {
    type Error = api::Error;

    fn try_from(connection: &model::Connection) -> Result<Self, Self::Error> {
        api::Podman::new(connection.url()).map(|podman| {
            glib::Object::new(&[
                ("connection", connection),
                ("podman", &BoxedPodman::from(podman)),
            ])
            .expect("Failed to create Client")
        })
    }
}

impl Client {
    pub(crate) fn podman(&self) -> &BoxedPodman {
        self.imp().podman.get().unwrap()
    }

    pub(crate) fn connection(&self) -> &model::Connection {
        self.imp().connection.get().unwrap()
    }

    pub(crate) fn image_list(&self) -> &model::ImageList {
        self.imp()
            .image_list
            .get_or_init(|| model::ImageList::from(Some(self)))
    }

    pub(crate) fn container_list(&self) -> &model::ContainerList {
        self.imp()
            .container_list
            .get_or_init(|| model::ContainerList::from(Some(self)))
    }

    pub(crate) fn pod_list(&self) -> &model::PodList {
        self.imp()
            .pod_list
            .get_or_init(|| model::PodList::from(Some(self)))
    }

    pub(crate) fn pruning(&self) -> bool {
        self.imp().pruning.get()
    }

    fn set_pruning(&self, value: bool) {
        if self.pruning() == value {
            return;
        }
        self.imp().pruning.set(value);
        self.notify("pruning");
    }

    pub(crate) fn prune<F>(&self, opts: api::ImagePruneOpts, op: F)
    where
        F: FnOnce(api::Result<Option<Vec<api::PruneReport>>>) + 'static,
    {
        self.set_pruning(true);
        utils::do_async(
            {
                let podman = self.podman().clone();
                async move { podman.images().prune(&opts).await }
            },
            clone!(@weak self as obj => move |result| {
                match result.as_ref() {
                    Ok(_) => log::info!("All images have been pruned"),
                    Err(e) => log::error!("Error on pruning images: {e}"),
                }
                obj.set_pruning(false);
                op(result);
            }),
        );
    }

    pub(crate) fn check_service<T, E, F>(&self, op: T, err_op: E, finish_op: F)
    where
        T: FnOnce() + 'static,
        E: FnOnce(ClientError) + Clone + 'static,
        F: FnOnce(api::Error) + Clone + 'static,
    {
        utils::do_async(
            {
                let podman = self.podman().clone();
                async move { podman.ping().await }
            },
            clone!(@weak self as obj => move |result| match result {
                Ok(_) => {
                    obj.image_list().refresh({
                        let err_op = err_op.clone();
                        |err| {
                            err_op(ClientError {
                                err,
                                variant: ClientErrorVariant::Images,
                            })
                        }
                    });
                    obj.container_list().refresh({
                        let err_op = err_op.clone();
                        |err| {
                            err_op(ClientError {
                                err,
                                variant: ClientErrorVariant::Containers,
                            })
                        }
                    });
                    obj.pod_list().refresh({
                        let err_op = err_op.clone();
                        |err| {
                            err_op(ClientError {
                                err,
                                variant: ClientErrorVariant::Pods,
                            })
                        }
                    });

                    op();
                    obj.start_event_listener(err_op, finish_op);
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
        F: FnOnce(api::Error) + Clone + 'static,
    {
        utils::run_stream(
            self.podman().clone(),
            |podman| podman.events(&api::EventsOpts::builder().build()).boxed(),
            clone!(
                @weak self as obj => @default-return glib::Continue(false),
                move |result: api::Result<api::Event>|
            {
                glib::Continue(match result {
                    Ok(event) => {
                        log::debug!("Event: {event:?}");
                        match event.typ.as_str() {
                            "image" => obj.image_list().handle_event(event, {
                                let err_op = err_op.clone();
                                |err| {
                                    err_op(ClientError {
                                        err,
                                        variant: ClientErrorVariant::Images,
                                    })
                                }
                            }),
                            "container" => obj.container_list().handle_event(event, {
                                let err_op = err_op.clone();
                                |err| {
                                    err_op(ClientError {
                                        err,
                                        variant: ClientErrorVariant::Containers,
                                    })
                                }
                            }),
                            "pod" => obj.pod_list().handle_event(event, {
                                let err_op = err_op.clone();
                                |err| {
                                    err_op(ClientError {
                                        err,
                                        variant: ClientErrorVariant::Pods,
                                    })
                                }
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
}
