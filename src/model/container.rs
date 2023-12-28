use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use futures::Future;
use gettextrs::gettext;
use glib::clone;
use glib::once_cell::sync::Lazy as SyncLazy;
use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::subclass::Signal;
use glib::Properties;
use gtk::glib;

use crate::model;
use crate::monad_boxed_type;
use crate::podman;
use crate::utils;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ContainerStatus")]
pub(crate) enum Status {
    Configured,
    Created,
    Dead,
    Exited,
    Initialized,
    Paused,
    Removing,
    Restarting,
    Running,
    Stopped,
    Stopping,
    #[default]
    Unknown,
}

impl FromStr for Status {
    type Err = Self;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "configured" => Self::Configured,
            "created" => Self::Created,
            "dead" => Self::Dead,
            "exited" => Self::Exited,
            "initialized" => Self::Initialized,
            "paused" => Self::Paused,
            "removing" => Self::Removing,
            "restarting" => Self::Restarting,
            "running" => Self::Running,
            "stopped" => Self::Stopped,
            "stopping" => Self::Stopping,
            _ => return Err(Self::Unknown),
        })
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Configured => gettext("Configured"),
                Self::Created => gettext("Created"),
                Self::Dead => gettext("Dead"),
                Self::Exited => gettext("Exited"),
                Self::Initialized => gettext("Initialized"),
                Self::Paused => gettext("Paused"),
                Self::Removing => gettext("Removing"),
                Self::Restarting => gettext("Restarting"),
                Self::Running => gettext("Running"),
                Self::Stopped => gettext("Stopped"),
                Self::Stopping => gettext("Stopping"),
                Self::Unknown => gettext("Unknown"),
            }
        )
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ContainerHealthStatus")]
pub(crate) enum HealthStatus {
    Starting,
    Healthy,
    Unhealthy,
    Unconfigured,
    #[default]
    Unknown,
}

impl FromStr for HealthStatus {
    type Err = Self;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "starting" => Self::Starting,
            "healthy" => Self::Healthy,
            "unhealthy" => Self::Unhealthy,
            "" => Self::Unconfigured,
            _ => return Err(Self::Unknown),
        })
    }
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Starting => gettext("Checking"),
                Self::Healthy => gettext("Healthy"),
                Self::Unhealthy => gettext("Unhealthy"),
                Self::Unconfigured => gettext("Unconfigured"),
                Self::Unknown => gettext("Unknown"),
            }
        )
    }
}

monad_boxed_type!(pub(crate) BoxedContainerStats(podman::models::ContainerStats) impls Debug, PartialEq is nullable);

mod imp {
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::Container)]
    pub(crate) struct Container {
        pub(super) inspection_observers: RefCell<
            Option<
                utils::AsyncObservers<
                    podman::Result<podman::models::ContainerInspectResponseLibpod>,
                >,
            >,
        >,
        pub(super) mounts: OnceCell<HashSet<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) container_list: glib::WeakRef<model::ContainerList>,
        #[property(get, set)]
        pub(super) action_ongoing: Cell<bool>,
        #[property(get, set, construct_only)]
        pub(super) created: OnceCell<i64>,
        #[property(get = Self::data, nullable)]
        pub(super) data: OnceCell<Option<model::ContainerData>>,
        #[property(get, set, construct, builder(HealthStatus::default()))]
        pub(super) health_status: Cell<HealthStatus>,
        #[property(get, set, construct_only)]
        pub(super) id: OnceCell<String>,
        #[property(get, set, nullable)]
        pub(super) image: glib::WeakRef<model::Image>,
        #[property(get, set, construct_only)]
        pub(super) image_id: OnceCell<String>,
        #[property(get, set, construct, nullable)]
        pub(super) image_name: RefCell<Option<String>>,
        #[property(get, set, construct_only)]
        pub(super) is_infra: Cell<bool>,
        #[property(get, set, construct)]
        pub(super) name: RefCell<String>,
        #[property(get, set = Self::set_pod, explicit_notify, nullable)]
        pub(super) pod: glib::WeakRef<model::Pod>,
        #[property(get = Self::pod_id, set, construct_only, nullable)]
        pub(super) pod_id: OnceCell<Option<String>>,
        #[property(get = Self::port, set, construct_only, nullable)]
        pub(super) port: OnceCell<Option<String>>,
        #[property(get, set, nullable)]
        pub(super) stats: RefCell<Option<BoxedContainerStats>>,
        #[property(get, set = Self::set_status, construct, explicit_notify, builder(Status::default()))]
        pub(super) status: Cell<Status>,
        #[property(get, set, construct)]
        pub(super) up_since: Cell<i64>,
        #[property(get = Self::volume_list)]
        pub(super) volume_list: OnceCell<model::ContainerVolumeList>,
        #[property(get)]
        pub(super) to_be_deleted: Cell<bool>,
        #[property(get, set)]
        pub(super) selected: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Container {
        const NAME: &'static str = "Container";
        type Type = super::Container;
        type Interfaces = (model::Selectable,);
    }

    impl ObjectImpl for Container {
        fn signals() -> &'static [Signal] {
            static SIGNALS: SyncLazy<Vec<Signal>> =
                SyncLazy::new(|| vec![Signal::builder("deleted").build()]);
            SIGNALS.as_ref()
        }

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

    impl Container {
        pub(super) fn data(&self) -> Option<model::ContainerData> {
            self.data.get().cloned().flatten()
        }

        pub(super) fn set_data(&self, data: podman::models::InspectContainerData) {
            let obj = &*self.obj();
            if let Some(old) = self.data() {
                old.update(data);
                return;
            }
            self.data
                .set(Some(model::ContainerData::from(data)))
                .unwrap();
            obj.notify_data();
        }

        pub(super) fn set_pod(&self, value: Option<&model::Pod>) {
            let obj = &*self.obj();
            if obj.pod().as_ref() == value {
                return;
            }
            if let Some(pod) = value {
                pod.inspect_and_update();
            }
            self.pod.set(value);
            obj.notify_pod();
        }

        pub(super) fn pod_id(&self) -> Option<String> {
            self.pod_id.get().cloned().flatten()
        }

        pub(super) fn port(&self) -> Option<String> {
            self.port.get().cloned().flatten()
        }

        pub(super) fn set_status(&self, value: Status) {
            let obj = &*self.obj();
            if obj.status() == value {
                return;
            }
            if let Some(pod) = obj.pod() {
                pod.inspect_and_update();
            }
            self.status.set(value);
            obj.notify_status();
        }

        pub(super) fn volume_list(&self) -> model::ContainerVolumeList {
            self.volume_list.get_or_init(Default::default).to_owned()
        }

        pub(super) fn set_to_be_deleted(&self, value: bool) {
            let obj = &*self.obj();
            if obj.to_be_deleted() == value {
                return;
            }
            self.to_be_deleted.set(value);
            obj.notify_to_be_deleted();
        }
    }
}

glib::wrapper! {
    pub(crate) struct Container(ObjectSubclass<imp::Container>) @implements model::Selectable;
}

impl Container {
    pub(crate) fn new(
        container_list: &model::ContainerList,
        list_container: podman::models::ListContainer,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("container-list", container_list)
            .property(
                "created",
                list_container.created.map(|dt| dt.timestamp()).unwrap_or(0),
            )
            .property(
                "health-status",
                health_status(list_container.status.as_deref()),
            )
            .property("id", list_container.id)
            .property("image-id", list_container.image_id)
            .property("image-name", list_container.image)
            .property("is-infra", list_container.is_infra.unwrap_or(false))
            .property("name", &list_container.names.unwrap()[0])
            .property("pod-id", list_container.pod)
            .property(
                "port",
                list_container
                    .ports
                    .unwrap_or_default()
                    .first()
                    .and_then(|mapping| {
                        mapping.host_port.map(|host_port| {
                            format!(
                                "{host_port}/{}",
                                mapping.protocol.as_deref().unwrap_or("tcp")
                            )
                        })
                    }),
            )
            .property("status", status(list_container.state.as_deref()))
            .property("up-since", list_container.started_at.unwrap())
            .build();

        obj.imp()
            .mounts
            .set(HashSet::from_iter(
                list_container.mounts.unwrap_or_default(),
            ))
            .unwrap();
        obj
    }

    pub(crate) fn mounts(&self) -> &HashSet<String> {
        self.imp().mounts.get().unwrap()
    }

    pub(crate) fn update(&self, list_container: podman::models::ListContainer) {
        self.set_action_ongoing(false);
        self.set_health_status(health_status(list_container.status.as_deref()));
        self.set_image_name(list_container.image);
        self.set_name(list_container.names.unwrap()[0].clone());
        self.set_status(status(list_container.state.as_deref()));
        self.set_up_since(list_container.started_at.unwrap());
    }

    pub(crate) fn inspect<F>(&self, op: F)
    where
        F: Fn(Result<model::Container, podman::Error>) + 'static,
    {
        if let Some(observers) = self.imp().inspection_observers.borrow().as_ref() {
            observers.add(clone!(@weak self as obj => move |result| match result {
                Ok(_) => op(Ok(obj)),
                Err(e) => {
                    log::error!("Error on inspecting container '{}': {e}", obj.id());
                    op(Err(e));
                }
            }));

            return;
        }

        let observers = utils::do_async_with_observers(
            {
                let container = self.api().unwrap();
                async move { container.inspect().await }
            },
            clone!(@weak self as obj => move |result| {
                let imp = obj.imp();

                imp.inspection_observers.replace(None);

                match result {
                    Ok(data) => {
                        imp.set_data(data);
                        op(Ok(obj));
                    },
                    Err(e) => {
                        log::error!("Error on inspecting container '{}': {e}", obj.id());
                        op(Err(e));
                    }
                }
            }),
        );

        self.imp().inspection_observers.replace(Some(observers));
    }

    fn action<Fut, FutOp, ResOp>(&self, name: &'static str, fut_op: FutOp, res_op: ResOp)
    where
        Fut: Future<Output = podman::Result<()>> + Send,
        FutOp: FnOnce(podman::api::Container) -> Fut + Send + 'static,
        ResOp: FnOnce(podman::Result<()>) + 'static,
    {
        if let Some(container) = self.api() {
            if self.action_ongoing() {
                return;
            }

            // This will be either set back to `false` in `Self::update` or in case of an error.
            self.set_action_ongoing(true);

            log::info!("Container <{}>: {name}…'", self.id());

            utils::do_async(
                async move { fut_op(container).await },
                clone!(@weak self as obj => move |result| {
                    match &result {
                        Ok(_) => {
                            log::info!(
                                "Container <{}>: {name} has finished",
                                obj.id()
                            );
                        }
                        Err(e) => {
                            log::error!(
                                "Container <{}>: Error while {name}: {e}",
                                obj.id(),
                            );
                            obj.set_action_ongoing(false);
                        }
                    }
                    res_op(result)
                }),
            );
        }
    }

    pub(crate) fn start<F>(&self, op: F)
    where
        F: FnOnce(podman::Result<()>) + 'static,
    {
        self.action(
            "starting",
            |container| async move { container.start(None).await },
            op,
        );
    }

    pub(crate) fn stop<F>(&self, force: bool, op: F)
    where
        F: FnOnce(podman::Result<()>) + 'static,
    {
        self.action(
            if force { "force stopping" } else { "stopping" },
            move |container| async move {
                if force {
                    container.kill().await
                } else {
                    container.stop(&Default::default()).await
                }
            },
            op,
        );
    }

    pub(crate) fn restart<F>(&self, force: bool, op: F)
    where
        F: FnOnce(podman::Result<()>) + 'static,
    {
        self.action(
            if force {
                "restarting"
            } else {
                "force restarting"
            },
            move |container| async move {
                if force {
                    container.restart_with_timeout(0).await
                } else {
                    container.restart().await
                }
            },
            op,
        );
    }

    pub(crate) fn pause<F>(&self, op: F)
    where
        F: FnOnce(podman::Result<()>) + 'static,
    {
        self.action(
            "pausing",
            |container| async move { container.pause().await },
            op,
        );
    }

    pub(crate) fn resume<F>(&self, op: F)
    where
        F: FnOnce(podman::Result<()>) + 'static,
    {
        self.action(
            "resuming",
            |container| async move { container.unpause().await },
            op,
        );
    }

    pub(crate) fn rename<F>(&self, new_name: String, op: F)
    where
        F: FnOnce(podman::Result<()>) + 'static,
    {
        self.action(
            "renaming",
            |container| async move { container.rename(new_name).await },
            op,
        );
    }

    pub(crate) fn delete<F>(&self, force: bool, op: F)
    where
        F: FnOnce(podman::Result<()>) + 'static,
    {
        if !self.action_ongoing() {
            self.imp().set_to_be_deleted(true);
        }
        self.action(
            if force { "force deleting" } else { "deleting" },
            move |container| async move {
                container
                    .delete(
                        &podman::opts::ContainerDeleteOpts::builder()
                            .force(force)
                            .build(),
                    )
                    .await
            },
            clone!(@weak self as obj => move |result| {
                if result.is_err() {
                    obj.imp().set_to_be_deleted(false);
                }
                op(result)
            }),
        );
    }

    pub(super) fn on_deleted(&self) {
        if let Some(pod) = self.pod() {
            pod.inspect_and_update();
        }
        self.emit_by_name::<()>("deleted", &[]);
    }
    pub(crate) fn connect_deleted<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("deleted", true, move |values| {
            f(&values[0].get::<Self>().unwrap());

            None
        })
    }

    pub(crate) fn can_start(&self) -> bool {
        matches!(
            self.status(),
            Status::Configured
                | Status::Created
                | Status::Exited
                | Status::Initialized
                | Status::Stopped
        )
    }

    pub(crate) fn can_stop(&self) -> bool {
        matches!(self.status(), Status::Running)
    }

    pub(crate) fn can_kill(&self) -> bool {
        !self.can_start()
    }

    pub(crate) fn can_restart(&self) -> bool {
        matches!(self.status(), Status::Running)
    }

    pub(crate) fn can_pause(&self) -> bool {
        matches!(self.status(), Status::Running)
    }

    pub(crate) fn can_resume(&self) -> bool {
        matches!(self.status(), Status::Paused)
    }

    pub(crate) fn can_delete(&self) -> bool {
        !matches!(self.status(), Status::Running | Status::Paused)
    }

    pub(crate) fn api(&self) -> Option<podman::api::Container> {
        self.container_list()
            .unwrap()
            .client()
            .map(|client| podman::api::Container::new(client.podman().deref().clone(), self.id()))
    }
}

fn status(s: Option<&str>) -> Status {
    s.map(|s| match Status::from_str(s) {
        Ok(status) => status,
        Err(status) => {
            log::warn!("Unknown container status: {s}");
            status
        }
    })
    .unwrap_or_default()
}

fn health_status(s: Option<&str>) -> HealthStatus {
    s.map(|s| match HealthStatus::from_str(s) {
        Ok(status) => status,
        Err(status) => {
            log::warn!("Unknown container health status: {s}");
            status
        }
    })
    .unwrap_or_default()
}
