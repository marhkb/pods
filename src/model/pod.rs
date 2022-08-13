use std::cell::Cell;
use std::cell::RefCell;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use futures::Future;
use futures::TryFutureExt;
use gettextrs::gettext;
use gtk::glib::clone;
use gtk::glib::subclass::Signal;
use gtk::glib::WeakRef;
use gtk::glib::{self};
use gtk::prelude::ObjectExt;
use gtk::prelude::StaticType;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::api;
use crate::model;
use crate::utils;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "PodStatus")]
pub(crate) enum Status {
    Configured,
    Created,
    Dead,
    Degraded,
    Exited,
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
            "Configured" => Self::Configured,
            "Created" => Self::Created,
            "Dead" => Self::Dead,
            "Degraded" => Self::Degraded,
            "Exited" => Self::Exited,
            "Paused" => Self::Paused,
            "Removing" => Self::Removing,
            "Restarting" => Self::Restarting,
            "Running" => Self::Running,
            "Stopped" => Self::Stopped,
            "Stopping" => Self::Stopping,
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
                Self::Degraded => gettext("Degraded"),
                Self::Exited => gettext("Exited"),
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

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct Pod {
        pub(super) pod_list: WeakRef<model::PodList>,
        pub(super) container_list: OnceCell<model::SimpleContainerList>,

        pub(super) action_ongoing: Cell<bool>,

        pub(super) created: OnceCell<i64>,
        pub(super) hostname: OnceCell<String>,
        pub(super) id: OnceCell<String>,
        pub(super) name: RefCell<String>,
        pub(super) num_containers: Cell<i64>,
        pub(super) status: Cell<Status>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Pod {
        const NAME: &'static str = "Pod";
        type Type = super::Pod;
    }

    impl ObjectImpl for Pod {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("deleted", &[], <()>::static_type().into()).build()]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "pod-list",
                        "Pod List",
                        "The parent pod list",
                        model::PodList::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecObject::new(
                        "container-list",
                        "Container List",
                        "The list of containers associated with this Image",
                        model::SimpleContainerList::static_type(),
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecBoolean::new(
                        "action-ongoing",
                        "Action Ongoing",
                        "Whether an action (starting, stopping, etc.) is currently ongoing",
                        false,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecBoolean::new(
                        "deleted",
                        "Deleted",
                        "Whether this pod is deleted",
                        false,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecInt64::new(
                        "created",
                        "Created",
                        "The time when this pod was created",
                        i64::MIN,
                        i64::MAX,
                        0,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "hostname",
                        "Hostname",
                        "The hostname of this pod",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "id",
                        "Id",
                        "The id of this pod",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "name",
                        "Name",
                        "The name of this pod",
                        Option::default(),
                        glib::ParamFlags::READWRITE
                            | glib::ParamFlags::CONSTRUCT
                            | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecInt64::new(
                        "num-containers",
                        "Num Containers",
                        "The number of containers in this pod",
                        0,
                        i64::MAX,
                        0,
                        glib::ParamFlags::READWRITE
                            | glib::ParamFlags::CONSTRUCT
                            | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecEnum::new(
                        "status",
                        "Status",
                        "The status of this pod",
                        Status::static_type(),
                        Status::default() as i32,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "pod-list" => self.pod_list.set(value.get().unwrap()),
                "action-ongoing" => obj.set_action_ongoing(value.get().unwrap()),
                "created" => self.created.set(value.get().unwrap()).unwrap(),
                "hostname" => self.hostname.set(value.get().unwrap()).unwrap(),
                "id" => self.id.set(value.get().unwrap()).unwrap(),
                "name" => obj.set_name(value.get().unwrap()),
                "num-containers" => obj.set_num_containers(value.get().unwrap()),
                "status" => obj.set_status(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "pod-list" => obj.pod_list().to_value(),
                "container-list" => obj.container_list().to_value(),
                "action-ongoing" => obj.action_ongoing().to_value(),
                "created" => obj.created().to_value(),
                "hostname" => obj.hostname().to_value(),
                "id" => obj.id().to_value(),
                "name" => obj.name().to_value(),
                "num-containers" => obj.num_containers().to_value(),
                "status" => obj.status().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct Pod(ObjectSubclass<imp::Pod>);
}

impl Pod {
    pub(crate) fn new(
        pod_list: &model::PodList,
        inspect_response: api::LibpodPodInspectResponse,
    ) -> Self {
        glib::Object::new(&[
            ("pod-list", pod_list),
            (
                "created",
                &inspect_response
                    .created
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0),
            ),
            ("hostname", &inspect_response.hostname.unwrap_or_default()),
            ("id", &inspect_response.id),
            ("name", &inspect_response.name),
            (
                "num-containers",
                &inspect_response.num_containers.unwrap_or(0),
            ),
            ("status", &status(inspect_response.state.as_deref())),
        ])
        .expect("Failed to create Pod")
    }

    pub(crate) fn update(&self, inspect_response: api::LibpodPodInspectResponse) {
        self.set_action_ongoing(false);
        self.set_name(inspect_response.name.unwrap_or_default());
        self.set_num_containers(inspect_response.num_containers.unwrap_or(0));
        self.set_status(status(inspect_response.state.as_deref()));
    }

    pub(crate) fn inspect_and_update(&self) {
        utils::do_async(
            {
                let pod = self.api_pod().unwrap();
                async move { pod.inspect().await }
            },
            clone!(@weak self as obj => move |result| match result {
                Ok(inspect_response) => obj.update(inspect_response),
                Err(e) => log::error!("Error on inspecting pod '{}': {e}", obj.id()),
            }),
        );
    }

    pub(crate) fn pod_list(&self) -> Option<model::PodList> {
        self.imp().pod_list.upgrade()
    }

    pub(crate) fn container_list(&self) -> &model::SimpleContainerList {
        self.imp().container_list.get_or_init(Default::default)
    }

    pub(crate) fn action_ongoing(&self) -> bool {
        self.imp().action_ongoing.get()
    }

    pub(crate) fn set_action_ongoing(&self, value: bool) {
        if self.action_ongoing() == value {
            return;
        }
        self.imp().action_ongoing.replace(value);
        self.notify("action-ongoing");
    }

    pub(crate) fn created(&self) -> i64 {
        *self.imp().created.get().unwrap()
    }

    pub(crate) fn hostname(&self) -> &str {
        self.imp().hostname.get().unwrap()
    }

    pub(crate) fn id(&self) -> &str {
        self.imp().id.get().unwrap()
    }

    pub(crate) fn name(&self) -> String {
        self.imp().name.borrow().clone()
    }

    pub(crate) fn set_name(&self, value: String) {
        if self.name() == value {
            return;
        }
        self.imp().name.replace(value);
        self.notify("name");
    }

    pub(crate) fn num_containers(&self) -> i64 {
        self.imp().num_containers.get()
    }

    fn set_num_containers(&self, value: i64) {
        if self.num_containers() == value {
            return;
        }
        self.imp().num_containers.replace(value);
        self.notify("num-containers");
    }

    pub(crate) fn status(&self) -> Status {
        self.imp().status.get()
    }

    pub(crate) fn set_status(&self, value: Status) {
        if self.status() == value {
            return;
        }
        self.imp().status.set(value);
        self.notify("status");
    }

    pub(super) fn emit_deleted(&self) {
        self.emit_by_name::<()>("deleted", &[]);
    }

    pub(crate) fn connect_deleted<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("deleted", true, move |values| {
            f(&values[0].get::<Self>().unwrap());

            None
        })
    }
}

impl Pod {
    fn action<Fut, FutOp, ResOp>(&self, name: &'static str, fut_op: FutOp, res_op: ResOp)
    where
        Fut: Future<Output = api::Result<()>> + Send,
        FutOp: FnOnce(api::Pod) -> Fut + Send + 'static,
        ResOp: FnOnce(api::Result<()>) + 'static,
    {
        if let Some(pod) = self.api_pod() {
            if self.action_ongoing() {
                return;
            }

            // This will be either set back to `false` in `Self::update` or in case of an error.
            self.set_action_ongoing(true);

            log::info!("Pod <{}>: {name}…'", self.id());

            utils::do_async(
                async move { fut_op(pod).await },
                clone!(@weak self as obj => move |result| {
                    match &result {
                        Ok(_) => {
                            log::info!(
                                "Pod <{}>: {name} has finished",
                                obj.id()
                            );
                        }
                        Err(e) => {
                            log::error!(
                                "Pod <{}>: Error while {name}: {e}",
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
        F: FnOnce(api::Result<()>) + 'static,
    {
        self.action(
            "starting",
            |pod| async move { pod.start().await.map(|_| ()) },
            op,
        );
    }

    pub(crate) fn stop<F>(&self, force: bool, op: F)
    where
        F: FnOnce(api::Result<()>) + 'static,
    {
        self.action(
            if force { "force stopping" } else { "stopping" },
            move |pod| async move {
                if force {
                    pod.kill().await.map(|_| ())
                } else {
                    pod.stop().await.map(|_| ())
                }
            },
            op,
        );
    }

    pub(crate) fn restart<F>(&self, force: bool, op: F)
    where
        F: FnOnce(api::Result<()>) + 'static,
    {
        self.action(
            if force {
                "force restarting"
            } else {
                "restarting"
            },
            move |pod| async move {
                if force {
                    pod.kill().and_then(|_| pod.start()).await.map(|_| ())
                } else {
                    pod.restart().await.map(|_| ())
                }
            },
            op,
        );
    }

    pub(crate) fn pause<F>(&self, op: F)
    where
        F: FnOnce(api::Result<()>) + 'static,
    {
        self.action(
            "pausing",
            |pod| async move { pod.pause().await.map(|_| ()) },
            op,
        );
    }

    pub(crate) fn resume<F>(&self, op: F)
    where
        F: FnOnce(api::Result<()>) + 'static,
    {
        self.action(
            "resuming",
            |pod| async move { pod.unpause().await.map(|_| ()) },
            op,
        );
    }

    pub(crate) fn delete<F>(&self, force: bool, op: F)
    where
        F: FnOnce(api::Result<()>) + 'static,
    {
        self.action(
            if force { "force deleting" } else { "deleting" },
            move |pod| async move {
                if force {
                    pod.remove().await
                } else {
                    pod.delete().await
                }
                .map(|_| ())
            },
            op,
        );
    }

    pub(crate) fn api_pod(&self) -> Option<api::Pod> {
        self.pod_list()
            .unwrap()
            .client()
            .map(|client| api::Pod::new(client.podman().deref().clone(), self.id()))
    }
}

fn status(state: Option<&str>) -> Status {
    state.map_or_else(Status::default, |s| match Status::from_str(s) {
        Ok(status) => status,
        Err(status) => {
            log::warn!("Unknown container status: {s}");
            status
        }
    })
}
