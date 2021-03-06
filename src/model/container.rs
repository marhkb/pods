use std::cell::Cell;
use std::cell::RefCell;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use futures::Future;
use futures::StreamExt;
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
use crate::monad_boxed_type;
use crate::utils;

#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ContainerStatus")]
pub(crate) enum Status {
    Configured,
    Created,
    Dead,
    Exited,
    Paused,
    Removing,
    Restarting,
    Running,
    Stopped,
    Stopping,
    Unknown,
}

impl Default for Status {
    fn default() -> Self {
        Self::Unknown
    }
}

impl FromStr for Status {
    type Err = Self;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "configured" => Self::Configured,
            "created" => Self::Created,
            "dead" => Self::Dead,
            "exited" => Self::Exited,
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

monad_boxed_type!(pub(crate) BoxedContainerStats(api::LibpodContainerStats) impls Debug, PartialEq is nullable);

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct Container {
        pub(super) container_list: WeakRef<model::ContainerList>,

        pub(super) action_ongoing: Cell<bool>,

        pub(super) created: OnceCell<i64>,
        pub(super) id: OnceCell<String>,
        pub(super) image: WeakRef<model::Image>,
        pub(super) image_id: OnceCell<String>,
        pub(super) image_name: RefCell<Option<String>>,
        pub(super) name: RefCell<Option<String>>,
        pub(super) port_bindings: OnceCell<utils::BoxedStringVec>,
        pub(super) stats: RefCell<Option<BoxedContainerStats>>,
        pub(super) status: Cell<Status>,
        pub(super) up_since: Cell<i64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Container {
        const NAME: &'static str = "Container";
        type Type = super::Container;
    }

    impl ObjectImpl for Container {
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
                        "container-list",
                        "Container List",
                        "The parent container list",
                        model::ContainerList::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
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
                        "Whether this container is deleted",
                        false,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecInt64::new(
                        "created",
                        "Created",
                        "The time when this container was created",
                        i64::MIN,
                        i64::MAX,
                        0,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "id",
                        "Id",
                        "The id of this container",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecObject::new(
                        "image",
                        "Image",
                        "The image of this container",
                        model::Image::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecString::new(
                        "image-id",
                        "Image Id",
                        "The image id of this container",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "image-name",
                        "Image Name",
                        "The name of the image of this container",
                        Option::default(),
                        glib::ParamFlags::READWRITE
                            | glib::ParamFlags::CONSTRUCT
                            | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecString::new(
                        "name",
                        "Name",
                        "The name of this container",
                        Option::default(),
                        glib::ParamFlags::READWRITE
                            | glib::ParamFlags::CONSTRUCT
                            | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecBoxed::new(
                        "port-bindings",
                        "Port Bindings",
                        "The port bindings of this container",
                        utils::BoxedStringVec::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecBoxed::new(
                        "stats",
                        "Stats",
                        "The statistics of this container",
                        BoxedContainerStats::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecEnum::new(
                        "status",
                        "Status",
                        "The status of this container",
                        Status::static_type(),
                        Status::default() as i32,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecInt64::new(
                        "up-since",
                        "Up Since",
                        "The time since the container is running",
                        i64::MIN,
                        i64::MAX,
                        0,
                        glib::ParamFlags::READWRITE
                            | glib::ParamFlags::CONSTRUCT
                            | glib::ParamFlags::EXPLICIT_NOTIFY,
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
                "container-list" => self.container_list.set(value.get().unwrap()),
                "action-ongoing" => obj.set_action_ongoing(value.get().unwrap()),
                "created" => self.created.set(value.get().unwrap()).unwrap(),
                "id" => self.id.set(value.get().unwrap()).unwrap(),
                "image" => obj.set_image(value.get().unwrap()),
                "image-id" => self.image_id.set(value.get().unwrap()).unwrap(),
                "image-name" => obj.set_image_name(value.get().unwrap()),
                "name" => obj.set_name(value.get().unwrap()),
                "port-bindings" => self.port_bindings.set(value.get().unwrap()).unwrap(),
                "stats" => obj.set_stats(value.get().unwrap()),
                "status" => obj.set_status(value.get().unwrap()),
                "up-since" => obj.set_up_since(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container-list" => obj.container_list().to_value(),
                "action-ongoing" => obj.action_ongoing().to_value(),
                "created" => obj.created().to_value(),
                "id" => obj.id().to_value(),
                "image" => obj.image().to_value(),
                "image-id" => obj.image_id().to_value(),
                "image-name" => obj.image_name().to_value(),
                "name" => obj.name().to_value(),
                "port-bindings" => obj.port_bindings().to_value(),
                "stats" => obj.stats().to_value(),
                "status" => obj.status().to_value(),
                "up-since" => obj.up_since().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }
}

glib::wrapper! {
    pub(crate) struct Container(ObjectSubclass<imp::Container>);
}

impl Container {
    pub(crate) fn new(
        container_list: &model::ContainerList,
        inspect_response: api::LibpodContainerInspectResponse,
    ) -> Self {
        glib::Object::new(&[
            ("container-list", container_list),
            (
                "created",
                &inspect_response
                    .created
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0),
            ),
            ("id", &inspect_response.id),
            ("image-id", &inspect_response.image),
            ("image-name", &inspect_response.image_name),
            ("name", &inspect_response.name),
            (
                "port-bindings",
                &utils::BoxedStringVec::from(
                    inspect_response
                        .host_config
                        .and_then(|config| config.port_bindings)
                        .map(|bindings| {
                            bindings
                                .into_values()
                                .flatten()
                                .flat_map(|host_ports| {
                                    host_ports.into_iter().map(|host_port| {
                                        format!(
                                            "{}:{}",
                                            {
                                                let ip = host_port.host_ip.unwrap_or_default();
                                                if ip.is_empty() {
                                                    "127.0.0.1".to_string()
                                                } else {
                                                    ip
                                                }
                                            },
                                            host_port.host_port.unwrap()
                                        )
                                    })
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default(),
                ),
            ),
            ("status", &status(inspect_response.state.as_ref())),
            ("up-since", &up_since(inspect_response.state.as_ref())),
        ])
        .expect("Failed to create Container")
    }

    pub(crate) fn update(&self, inspect_response: api::LibpodContainerInspectResponse) {
        self.set_action_ongoing(false);
        self.set_image_name(inspect_response.image_name);
        self.set_name(inspect_response.name);
        self.set_status(status(inspect_response.state.as_ref()));
        self.set_up_since(up_since(inspect_response.state.as_ref()));
    }

    pub(crate) fn container_list(&self) -> Option<model::ContainerList> {
        self.imp().container_list.upgrade()
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

    pub(crate) fn id(&self) -> Option<&str> {
        self.imp().id.get().map(String::as_str)
    }

    pub(crate) fn image(&self) -> Option<model::Image> {
        self.imp().image.upgrade()
    }

    pub(crate) fn set_image(&self, value: Option<&model::Image>) {
        if self.image().as_ref() == value {
            return;
        }
        self.imp().image.set(value);
        self.notify("image");
    }

    pub(crate) fn image_id(&self) -> Option<&str> {
        self.imp().image_id.get().map(String::as_str)
    }

    pub(crate) fn image_name(&self) -> Option<String> {
        self.imp().image_name.borrow().clone()
    }

    pub(crate) fn set_image_name(&self, value: Option<String>) {
        if self.image_name() == value {
            return;
        }
        self.imp().image_name.replace(value);
        self.notify("image-name");
    }

    pub(crate) fn name(&self) -> Option<String> {
        self.imp().name.borrow().clone()
    }

    pub(crate) fn set_name(&self, value: Option<String>) {
        if self.name() == value {
            return;
        }
        self.imp().name.replace(value);
        self.notify("name");
    }

    pub(crate) fn port_bindings(&self) -> &utils::BoxedStringVec {
        self.imp().port_bindings.get().unwrap()
    }

    pub(crate) fn stats(&self) -> Option<BoxedContainerStats> {
        self.imp().stats.borrow().clone()
    }

    pub fn set_stats(&self, value: Option<BoxedContainerStats>) {
        if self.stats() == value {
            return;
        }
        self.imp().stats.replace(value);
        self.notify("stats");
    }

    pub(crate) fn status(&self) -> Status {
        self.imp().status.get()
    }

    pub(crate) fn set_status(&self, value: Status) {
        if self.status() == value {
            return;
        }
        if value == Status::Running {
            self.run_stats_stream();
        }
        self.imp().status.set(value);
        self.notify("status");
    }

    pub(crate) fn up_since(&self) -> i64 {
        self.imp().up_since.get()
    }

    pub(crate) fn set_up_since(&self, value: i64) {
        if self.up_since() == value {
            return;
        }
        self.imp().up_since.set(value);
        self.notify("up-since");
    }
}

impl Container {
    fn action<Fut, FutOp, ResOp>(&self, name: &'static str, fut_op: FutOp, res_op: ResOp)
    where
        Fut: Future<Output = api::Result<()>> + Send,
        FutOp: FnOnce(api::Container) -> Fut + Send + 'static,
        ResOp: FnOnce(api::Result<()>) + 'static,
    {
        if let Some(container) = self.api_container() {
            if self.action_ongoing() {
                return;
            }

            // This will be either set back to `false` in `Self::update` or in case of an error.
            self.set_action_ongoing(true);

            log::info!("Container <{}>: {name}???'", self.id().unwrap_or_default());

            utils::do_async(
                async move { fut_op(container).await },
                clone!(@weak self as obj => move |result| {
                    match &result {
                        Ok(_) => {
                            log::info!(
                                "Container <{}>: {name} has finished",
                                obj.id().unwrap_or_default()
                            );
                        }
                        Err(e) => {
                            log::error!(
                                "Container <{}>: Error while {name}: {e}",
                                obj.id().unwrap_or_default(),
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
            |container| async move { container.start(None).await },
            op,
        );
    }

    pub(crate) fn stop<F>(&self, force: bool, op: F)
    where
        F: FnOnce(api::Result<()>) + 'static,
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
        F: FnOnce(api::Result<()>) + 'static,
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
        F: FnOnce(api::Result<()>) + 'static,
    {
        self.action(
            "pausing",
            |container| async move { container.pause().await },
            op,
        );
    }

    pub(crate) fn resume<F>(&self, op: F)
    where
        F: FnOnce(api::Result<()>) + 'static,
    {
        self.action(
            "resuming",
            |container| async move { container.unpause().await },
            op,
        );
    }

    pub(crate) fn rename<F>(&self, new_name: String, op: F)
    where
        F: FnOnce(api::Result<()>) + 'static,
    {
        self.action(
            "renaming",
            |container| async move { container.rename(new_name).await },
            op,
        );
    }

    pub(crate) fn commit<F>(&self, op: F)
    where
        F: FnOnce(api::Result<()>) + 'static,
    {
        self.action(
            "committing",
            |container| async move { container.commit(&Default::default()).await },
            op,
        );
    }

    pub(crate) fn delete<F>(&self, force: bool, op: F)
    where
        F: FnOnce(api::Result<()>) + 'static,
    {
        self.action(
            if force { "force deleting" } else { "deleting" },
            move |container| async move {
                container
                    .delete(&api::ContainerDeleteOpts::builder().force(force).build())
                    .await
            },
            op,
        );
    }

    fn run_stats_stream(&self) {
        if let Some(container) = self.api_container() {
            utils::run_stream(
                container,
                |container| container.stats_stream(Some(1)).boxed(),
                clone!(
                    @weak self as obj => @default-return glib::Continue(false),
                    move |result: api::Result<api::LibpodContainerStatsResponse>|
                {
                    glib::Continue(match result {
                        Ok(stats) => {
                            obj.set_stats(
                                stats
                                    .stats
                                    .and_then(|mut stats| stats.pop())
                                    .map(BoxedContainerStats),
                            );
                            true
                        }
                        Err(_) => false,
                    })
                }),
            );
        }
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

    pub(crate) fn api_container(&self) -> Option<api::Container> {
        self.container_list().unwrap().client().map(|client| {
            api::Container::new(
                client.podman().deref().clone(),
                self.id().unwrap_or_default(),
            )
        })
    }
}

fn status(state: Option<&api::InspectContainerState>) -> Status {
    state
        .and_then(|state| state.status.as_ref())
        .map_or_else(Status::default, |s| match Status::from_str(s) {
            Ok(status) => status,
            Err(status) => {
                log::warn!("Unknown container status: {s}");
                status
            }
        })
}

fn up_since(state: Option<&api::InspectContainerState>) -> i64 {
    state
        .and_then(|state| state.started_at.map(|dt| dt.timestamp()))
        .unwrap_or(0)
}
