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
use gtk::glib::{self};
use gtk::prelude::ObjectExt;
use gtk::prelude::ParamSpecBuilderExt;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::model;
use crate::podman;
use crate::utils;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "PodStatus")]
pub(crate) enum Status {
    Created,
    Dead,
    Degraded,
    Error,
    Exited,
    Paused,
    Restarting,
    Running,
    Stopped,
    #[default]
    Unknown,
}

impl FromStr for Status {
    type Err = Self;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Created" => Self::Created,
            "Dead" => Self::Dead,
            "Degraded" => Self::Degraded,
            "Error" => Self::Error,
            "Exited" => Self::Exited,
            "Paused" => Self::Paused,
            "Restarting" => Self::Restarting,
            "Stopped" => Self::Stopped,
            "Running" => Self::Running,
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
                Self::Created => gettext("Created"),
                Self::Dead => gettext("Dead"),
                Self::Degraded => gettext("Degraded"),
                Self::Error => gettext("Error"),
                Self::Exited => gettext("Exited"),
                Self::Paused => gettext("Paused"),
                Self::Restarting => gettext("Restarting"),
                Self::Running => gettext("Running"),
                Self::Stopped => gettext("Stopped"),
                Self::Unknown => gettext("Unknown"),
            }
        )
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct Pod {
        pub(super) pod_list: glib::WeakRef<model::PodList>,
        pub(super) container_list: OnceCell<model::SimpleContainerList>,

        pub(super) action_ongoing: Cell<bool>,

        pub(super) created: OnceCell<i64>,
        pub(super) hostname: OnceCell<String>,
        pub(super) id: OnceCell<String>,
        pub(super) name: RefCell<String>,
        pub(super) num_containers: Cell<u64>,
        pub(super) status: Cell<Status>,

        pub(super) data: OnceCell<model::PodData>,
        pub(super) can_inspect: Cell<bool>,

        pub(super) to_be_deleted: Cell<bool>,

        pub(super) selected: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Pod {
        const NAME: &'static str = "Pod";
        type Type = super::Pod;
        type Interfaces = (model::Selectable,);
    }

    impl ObjectImpl for Pod {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("deleted").build()]);
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::PodList>("pod-list")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecObject::builder::<model::SimpleContainerList>("container-list")
                        .flags(glib::ParamFlags::READABLE)
                        .build(),
                    glib::ParamSpecBoolean::builder("action-ongoing")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecBoolean::builder("deleted")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecInt64::builder("created")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecString::builder("id")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecString::builder("name")
                        .flags(
                            glib::ParamFlags::READWRITE
                                | glib::ParamFlags::CONSTRUCT
                                | glib::ParamFlags::EXPLICIT_NOTIFY,
                        )
                        .build(),
                    glib::ParamSpecUInt64::builder("num-containers")
                        .flags(
                            glib::ParamFlags::READWRITE
                                | glib::ParamFlags::CONSTRUCT
                                | glib::ParamFlags::EXPLICIT_NOTIFY,
                        )
                        .build(),
                    glib::ParamSpecEnum::builder::<Status>("status", Status::default())
                        .flags(
                            glib::ParamFlags::READWRITE
                                | glib::ParamFlags::CONSTRUCT
                                | glib::ParamFlags::EXPLICIT_NOTIFY,
                        )
                        .build(),
                    glib::ParamSpecObject::builder::<model::PodData>("data")
                        .flags(glib::ParamFlags::READABLE)
                        .build(),
                    glib::ParamSpecBoolean::builder("to-be-deleted")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecBoolean::builder("selected").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.instance();
            match pspec.name() {
                "pod-list" => self.pod_list.set(value.get().unwrap()),
                "action-ongoing" => obj.set_action_ongoing(value.get().unwrap()),
                "created" => self.created.set(value.get().unwrap()).unwrap(),
                "hostname" => self.hostname.set(value.get().unwrap()).unwrap(),
                "id" => self.id.set(value.get().unwrap()).unwrap(),
                "name" => obj.set_name(value.get().unwrap()),
                "num-containers" => obj.set_num_containers(value.get().unwrap()),
                "status" => obj.set_status(value.get().unwrap()),
                "to-be-deleted" => obj.set_to_be_deleted(value.get().unwrap()),
                "selected" => self.selected.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.instance();
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
                "data" => obj.data().to_value(),
                "to-be-deleted" => obj.to_be_deleted().to_value(),
                "selected" => self.selected.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.can_inspect.set(true);
        }
    }
}

glib::wrapper! {
    pub(crate) struct Pod(ObjectSubclass<imp::Pod>) @implements model::Selectable;
}

impl Pod {
    pub(crate) fn new(pod_list: &model::PodList, report: podman::models::ListPodsReport) -> Self {
        glib::Object::new::<Self>(&[
            ("pod-list", pod_list),
            (
                "created",
                &report.created.map(|dt| dt.timestamp()).unwrap_or(0),
            ),
            ("id", &report.id.unwrap()),
            ("name", &report.name.unwrap()),
            (
                "num-containers",
                &report.containers.map(|c| c.len() as u64).unwrap_or(0),
            ),
            ("status", &status(report.status.as_deref())),
        ])
    }

    pub(crate) fn update(&self, report: podman::models::ListPodsReport) {
        self.set_action_ongoing(false);
        self.set_name(report.name.unwrap_or_default());
        self.set_num_containers(report.containers.map(|c| c.len() as u64).unwrap_or(0));
        self.set_status(status(report.status.as_deref()));
    }

    pub(crate) fn inspect_and_update(&self) {
        if let Some(pod_list) = self.pod_list() {
            pod_list.refresh(Some(self.id().to_owned()), |_| {});
        }
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

    pub(crate) fn num_containers(&self) -> u64 {
        self.imp().num_containers.get()
    }

    fn set_num_containers(&self, value: u64) {
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

    pub(crate) fn data(&self) -> Option<&model::PodData> {
        self.imp().data.get()
    }

    fn set_data(&self, value: model::PodData) {
        if self.data().is_some() {
            return;
        }
        self.imp().data.set(value).unwrap();
        self.notify("data");
    }

    pub(crate) fn inspect<F>(&self, op: F)
    where
        F: FnOnce(podman::Error) + 'static,
    {
        let imp = self.imp();
        if !imp.can_inspect.get() {
            return;
        }

        imp.can_inspect.set(false);

        utils::do_async(
            {
                let pod = self.api().unwrap();
                async move { pod.inspect().await }
            },
            clone!(@weak self as obj => move |result| {
                match result {
                    Ok(data) => obj.set_data(model::PodData::from(
                        data
                    )),
                    Err(e) => {
                        log::error!("Error on inspecting pod '{}': {e}", obj.id());
                        op(e);
                    },
                }
                obj.imp().can_inspect.set(true);
            }),
        );
    }

    pub(crate) fn to_be_deleted(&self) -> bool {
        self.imp().to_be_deleted.get()
    }

    fn set_to_be_deleted(&self, value: bool) {
        if self.to_be_deleted() == value {
            return;
        }
        self.imp().to_be_deleted.set(value);
        self.notify("to-be-deleted");
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
        Fut: Future<Output = podman::Result<()>> + Send,
        FutOp: FnOnce(podman::api::Pod) -> Fut + Send + 'static,
        ResOp: FnOnce(podman::Result<()>) + 'static,
    {
        if let Some(pod) = self.api() {
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
        F: FnOnce(podman::Result<()>) + 'static,
    {
        self.action(
            "starting",
            |pod| async move { pod.start().await.map(|_| ()) },
            op,
        );
    }

    pub(crate) fn stop<F>(&self, force: bool, op: F)
    where
        F: FnOnce(podman::Result<()>) + 'static,
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
        F: FnOnce(podman::Result<()>) + 'static,
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
        F: FnOnce(podman::Result<()>) + 'static,
    {
        self.action(
            "pausing",
            |pod| async move { pod.pause().await.map(|_| ()) },
            op,
        );
    }

    pub(crate) fn resume<F>(&self, op: F)
    where
        F: FnOnce(podman::Result<()>) + 'static,
    {
        self.action(
            "resuming",
            |pod| async move { pod.unpause().await.map(|_| ()) },
            op,
        );
    }

    pub(crate) fn delete<F>(&self, force: bool, op: F)
    where
        F: FnOnce(podman::Result<()>) + 'static,
    {
        if !self.action_ongoing() {
            self.set_to_be_deleted(true);
        }
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
            clone!(@weak self as obj => move |result| {
                if result.is_err() {
                    obj.set_to_be_deleted(false);
                }
                op(result)
            }),
        );
    }

    pub(crate) fn can_start(&self) -> bool {
        matches!(
            self.status(),
            Status::Created | Status::Exited | Status::Stopped
        )
    }

    pub(crate) fn can_stop(&self) -> bool {
        matches!(self.status(), Status::Running)
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

    pub(crate) fn api(&self) -> Option<podman::api::Pod> {
        self.pod_list()
            .unwrap()
            .client()
            .map(|client| podman::api::Pod::new(client.podman().deref().clone(), self.id()))
    }
}

fn status(state: Option<&str>) -> Status {
    state.map_or_else(Status::default, |s| match Status::from_str(s) {
        Ok(status) => status,
        Err(status) => {
            log::warn!("Unknown pod status: {s}");
            status
        }
    })
}
