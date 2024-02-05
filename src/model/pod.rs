use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::OnceLock;

use futures::prelude::*;
use futures::Future;
use gettextrs::gettext;
use glib::clone;
use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::subclass::Signal;
use glib::Properties;
use gtk::glib;

use crate::model;
use crate::model::AbstractContainerListExt;
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

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::Pod)]
    pub(crate) struct Pod {
        pub(super) inspection_observers:
            RefCell<Option<utils::AsyncObservers<podman::Result<podman::models::InspectPodData>>>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) pod_list: glib::WeakRef<model::PodList>,
        #[property(get = Self::container_list)]
        pub(super) container_list: OnceCell<model::SimpleContainerList>,
        #[property(get)]
        pub(super) infra_container: glib::WeakRef<model::Container>,
        #[property(get, set)]
        pub(super) action_ongoing: Cell<bool>,
        #[property(get, set, construct_only)]
        pub(super) created: OnceCell<i64>,
        #[property(get = Self::data, nullable)]
        pub(super) data: OnceCell<Option<model::PodData>>,
        #[property(get, set, construct_only)]
        pub(super) id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) name: OnceCell<String>,
        #[property(get, set)]
        pub(super) num_containers: Cell<u64>,
        #[property(get, set, construct, builder(Status::default()))]
        pub(super) status: Cell<Status>,
        #[property(get)]
        pub(super) to_be_deleted: Cell<bool>,
        #[property(get, set)]
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
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("deleted").build()])
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

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let handler_id_ref = Rc::new(RefCell::new(None));
            let handler_id = obj.container_list().connect_container_added(
                clone!(@weak obj, @strong handler_id_ref => move |list, container| if container.is_infra() {
                    list.disconnect(handler_id_ref.take().unwrap());

                    obj.imp().infra_container.set(Some(container));
                    obj.notify_infra_container();
                }),
            );
            handler_id_ref.set(Some(handler_id));
        }
    }

    impl Pod {
        pub(super) fn container_list(&self) -> model::SimpleContainerList {
            self.container_list.get_or_init(Default::default).to_owned()
        }

        pub(super) fn data(&self) -> Option<model::PodData> {
            self.data.get().cloned().flatten()
        }

        pub(super) fn set_data(&self, value: model::PodData) {
            if self.data().is_some() {
                return;
            }
            self.data.set(Some(value)).unwrap();
            self.obj().notify_data();
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
    pub(crate) struct Pod(ObjectSubclass<imp::Pod>) @implements model::Selectable;
}

impl Pod {
    pub(crate) fn new(pod_list: &model::PodList, report: podman::models::ListPodsReport) -> Self {
        glib::Object::builder()
            .property("pod-list", pod_list)
            .property(
                "created",
                report.created.map(|dt| dt.timestamp()).unwrap_or(0),
            )
            .property("id", report.id.unwrap())
            .property("name", report.name.unwrap())
            .property(
                "num-containers",
                report.containers.map(|c| c.len() as u64).unwrap_or(0),
            )
            .property("status", status(report.status.as_deref()))
            .build()
    }

    pub(crate) fn update(&self, report: podman::models::ListPodsReport) {
        self.set_action_ongoing(false);
        self.set_num_containers(report.containers.map(|c| c.len() as u64).unwrap_or(0));
        self.set_status(status(report.status.as_deref()));
    }

    pub(crate) fn inspect_and_update(&self) {
        if let Some(pod_list) = self.pod_list() {
            pod_list.refresh(Some(self.id()), |_| {});
        }
    }

    pub(crate) fn inspect<F>(&self, op: F)
    where
        F: Fn(Result<model::Pod, podman::Error>) + 'static,
    {
        if let Some(observers) = self.imp().inspection_observers.borrow().as_ref() {
            observers.add(clone!(@weak self as obj => move |result| match result {
                Ok(_) => op(Ok(obj)),
                Err(e) => {
                    log::error!("Error on inspecting pod '{}': {e}", obj.id());
                    op(Err(e));
                }
            }));

            return;
        }

        let observers = utils::do_async_with_observers(
            {
                let pod = self.api().unwrap();
                async move { pod.inspect().await }
            },
            clone!(@weak self as obj => move |result| {
                let imp = obj.imp();

                imp.inspection_observers.replace(None);

                match result {
                    Ok(data) => {
                        imp.set_data(model::PodData::from(data));
                        op(Ok(obj));
                    },
                    Err(e) => {
                        log::error!("Error on inspecting pod '{}': {e}", obj.id());
                        op(Err(e));
                    }
                }
            }),
        );

        self.imp().inspection_observers.replace(Some(observers));
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
            self.imp().set_to_be_deleted(true);
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
                    obj.imp().set_to_be_deleted(false);
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
