use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::sync::OnceLock;

use futures::Future;
use glib::Properties;
use glib::clone;
use glib::prelude::*;
use glib::subclass::Signal;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;
use crate::model;
use crate::monad_boxed_type;
use crate::rt;

monad_boxed_type!(pub(crate) BoxedMounts(Vec<engine::dto::Mount>) impls Debug, PartialEq is nullable);
monad_boxed_type!(pub(crate) BoxedContainerStats(engine::dto::ContainerStats) impls Debug, PartialEq is nullable);

mod imp {
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::Container)]
    pub(crate) struct Container {
        #[property(get, set, construct_only, nullable)]
        pub(super) container_list: glib::WeakRef<model::ContainerList>,

        #[property(get, set, nullable)]
        pub(super) image: glib::WeakRef<model::Image>,
        #[property(get, set = Self::set_pod, explicit_notify, nullable)]
        pub(super) pod: glib::WeakRef<model::Pod>,
        #[property(get = Self::volume_list)]
        pub(super) volume_list: OnceCell<model::ContainerVolumeList>,

        #[property(get, set, construct_only)]
        pub(super) created: OnceCell<i64>,
        #[property(get, set, construct_only)]
        pub(super) id: OnceCell<String>,
        #[property(get, set, construct, default)]
        pub(super) health_status: Cell<model::ContainerHealthStatus>,
        #[property(get, set, construct_only)]
        pub(super) image_id: OnceCell<String>,
        #[property(get, set, construct_only, nullable)]
        pub(super) image_name: RefCell<Option<String>>,
        #[property(get, set, construct_only)]
        pub(super) is_infra: OnceCell<bool>,
        #[property(get, set, construct_only)]
        pub(super) mounts: OnceCell<BoxedMounts>,
        #[property(get, set, construct)]
        pub(super) name: RefCell<String>,
        #[property(get = Self::pod_id, set, construct_only, nullable)]
        pub(super) pod_id: OnceCell<Option<String>>,
        #[property(get, set, construct_only)]
        pub(super) ports: OnceCell<model::PortMappingList>,
        #[property(get, set = Self::set_status, construct, explicit_notify, default)]
        pub(super) status: Cell<model::ContainerStatus>,

        #[property(get = Self::details, set, nullable)]
        pub(super) details: OnceCell<Option<model::ContainerDetails>>,

        #[property(get, set, nullable)]
        pub(super) stats: RefCell<Option<BoxedContainerStats>>,

        #[property(get, set)]
        pub(super) selected: Cell<bool>,
        #[property(get, set)]
        pub(super) to_be_deleted: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Container {
        const NAME: &'static str = "Container";
        type Type = super::Container;
        type Interfaces = (model::Selectable,);
    }

    impl ObjectImpl for Container {
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
    }

    impl Container {
        pub(super) fn details(&self) -> Option<model::ContainerDetails> {
            self.details.get().cloned().flatten()
        }

        pub(super) fn set_pod(&self, value: Option<&model::Pod>) {
            let obj = &*self.obj();
            if obj.pod().as_ref() == value {
                return;
            }
            if let Some(pod) = value {
                pod.inspect_and_update(|e| log::error!("inspect pod: {e}"));
            }
            self.pod.set(value);
            obj.notify_pod();
        }

        pub(super) fn pod_id(&self) -> Option<String> {
            self.pod_id.get().cloned().flatten()
        }

        pub(super) fn set_status(&self, value: model::ContainerStatus) {
            let obj = &*self.obj();
            if obj.status() == value {
                return;
            }

            if let Some(pod) = obj.pod() {
                pod.inspect_and_update(|e| log::error!("inspect pod: {e}"));
            }

            self.status.set(value);
            obj.notify_status();
        }

        pub(super) fn volume_list(&self) -> model::ContainerVolumeList {
            self.volume_list.get_or_init(Default::default).to_owned()
        }
    }
}

glib::wrapper! {
    pub(crate) struct Container(ObjectSubclass<imp::Container>) @implements model::Selectable;
}

impl Container {
    pub(crate) fn new(container_list: &model::ContainerList, dto: engine::dto::Container) -> Self {
        match dto {
            engine::dto::Container::Summary(dto) => Self::new_from_summary(container_list, dto),
            engine::dto::Container::Inspection(dto) => {
                Self::new_from_inspection(container_list, dto)
            }
        }
    }

    pub(crate) fn new_from_summary(
        container_list: &model::ContainerList,
        dto: engine::dto::ContainerSummary,
    ) -> Self {
        Self::build(container_list, dto, |builder| builder)
    }

    pub(crate) fn new_from_inspection(
        container_list: &model::ContainerList,
        dto: engine::dto::ContainerInspection,
    ) -> Self {
        Self::build(container_list, dto.summary, |builder| {
            builder.property("details", model::ContainerDetails::from(dto.details))
        })
    }

    fn build<F>(
        container_list: &model::ContainerList,
        dto: engine::dto::ContainerSummary,
        op: F,
    ) -> Self
    where
        F: FnOnce(glib::object::ObjectBuilder<Self>) -> glib::object::ObjectBuilder<Self>,
    {
        op(glib::Object::builder()
            .property("container-list", container_list)
            .property("created", dto.created)
            .property(
                "health-status",
                model::ContainerHealthStatus::from(dto.health_status),
            )
            .property("id", dto.id)
            .property("image-id", dto.image_id)
            .property("image-name", dto.image_name)
            .property("is-infra", dto.is_infra)
            .property("mounts", BoxedMounts::from(dto.mounts))
            .property("name", dto.name)
            .property("pod-id", dto.pod_id)
            .property("ports", model::PortMappingList::from(dto.ports))
            .property("status", model::ContainerStatus::from(dto.status)))
        .build()
    }

    pub(crate) fn update(&self, dto: engine::dto::Container) {
        match dto {
            engine::dto::Container::Summary(dto) => self.update_from_summary(dto),
            engine::dto::Container::Inspection(dto) => self.update_from_inspection(dto),
        }
    }

    pub(crate) fn update_from_summary(&self, dto: engine::dto::ContainerSummary) {
        self.set_health_status(model::ContainerHealthStatus::from(dto.health_status));
        self.set_name(dto.name);
        self.set_status(model::ContainerStatus::from(dto.status));
    }

    pub(crate) fn update_from_inspection(&self, dto: engine::dto::ContainerInspection) {
        self.update_from_summary(dto.summary);

        match self.details() {
            Some(details) => details.update(dto.details),
            None => self.set_details(Some(model::ContainerDetails::from(dto.details))),
        }
    }

    pub(crate) fn inspect_and_update<F>(&self, op: F)
    where
        F: Fn(anyhow::Error) + 'static,
    {
        let Some(api) = self.api() else { return };

        rt::Promise::new(async move { api.inspect().await }).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |dto| match dto {
                Ok(dto) => obj.update_from_inspection(dto),
                Err(e) => op(e),
            }
        ));
    }

    pub(crate) fn api(&self) -> Option<engine::api::Container> {
        self.container_list()
            .and_then(|container_list| container_list.api())
            .map(|api| api.get(self.id()))
    }

    pub(super) fn on_deleted(&self) {
        if let Some(pod) = self.pod() {
            pod.inspect_and_update(|e| log::error!("inspect pod: {e}"));
        }
        self.emit_by_name::<()>("deleted", &[]);
    }

    pub(crate) fn connect_deleted<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("deleted", true, move |values| {
            f(&values[0].get::<Self>().unwrap());

            None
        })
    }

    pub(crate) fn has_pod(&self) -> bool {
        self.pod_id().filter(|id| !id.is_empty()).is_some()
    }
}

// Actions
impl Container {
    pub(crate) fn start<F>(&self, op: F)
    where
        F: FnOnce(anyhow::Result<()>) + 'static,
    {
        let prev_status = self.status();
        self.set_status(model::ContainerStatus::Starting);

        self.action(
            "starting",
            |container| async move { container.start().await },
            clone!(
                #[weak(rename_to = obj)]
                self,
                move |result| {
                    if result.is_err() {
                        obj.set_status(prev_status);
                    }
                    op(result);
                }
            ),
        );
    }

    pub(crate) fn stop<F>(&self, force: bool, op: F)
    where
        F: FnOnce(anyhow::Result<()>) + 'static,
    {
        let prev_status = self.status();
        self.set_status(model::ContainerStatus::Stopping);

        self.action(
            if force { "force stopping" } else { "stopping" },
            move |container| async move {
                if force {
                    container.kill().await
                } else {
                    container.stop().await
                }
            },
            clone!(
                #[weak(rename_to = obj)]
                self,
                move |result| {
                    if result.is_err() {
                        obj.set_status(prev_status);
                    }
                    op(result);
                }
            ),
        );
    }

    pub(crate) fn restart<F>(&self, force: bool, op: F)
    where
        F: FnOnce(anyhow::Result<()>) + 'static,
    {
        let prev_status = self.status();
        self.set_status(model::ContainerStatus::Restarting);

        self.action(
            if force {
                "restarting"
            } else {
                "force restarting"
            },
            move |container| async move { container.restart(force).await },
            clone!(
                #[weak(rename_to = obj)]
                self,
                move |result| {
                    if result.is_err() {
                        obj.set_status(prev_status);
                    }
                    op(result);
                }
            ),
        );
    }

    pub(crate) fn pause<F>(&self, op: F)
    where
        F: FnOnce(anyhow::Result<()>) + 'static,
    {
        self.action(
            "pausing",
            |container| async move { container.pause().await },
            op,
        );
    }

    pub(crate) fn resume<F>(&self, op: F)
    where
        F: FnOnce(anyhow::Result<()>) + 'static,
    {
        self.action(
            "resuming",
            |container| async move { container.unpause().await },
            op,
        );
    }

    pub(crate) fn rename<F>(&self, new_name: String, op: F)
    where
        F: FnOnce(anyhow::Result<()>) + 'static,
    {
        self.action(
            "renaming",
            |container| async move { container.rename(&new_name).await },
            op,
        );
    }

    pub(crate) fn remove<F>(&self, op: F)
    where
        F: FnOnce(anyhow::Result<()>) + 'static,
    {
        self.set_to_be_deleted(true);

        let force = self.status() == model::ContainerStatus::Running;

        self.action(
            if force { "force-removing" } else { "removing" },
            move |container| async move { container.remove(force).await },
            clone!(
                #[weak(rename_to = obj)]
                self,
                move |result| {
                    if result.is_err() {
                        obj.set_to_be_deleted(false);
                    }
                    op(result)
                }
            ),
        );
    }

    fn action<Fut, FutOp, ResOp>(&self, name: &'static str, fut_op: FutOp, res_op: ResOp)
    where
        Fut: Future<Output = anyhow::Result<()>> + Send,
        FutOp: FnOnce(engine::api::Container) -> Fut + Send + 'static,
        ResOp: FnOnce(anyhow::Result<()>) + 'static,
    {
        let Some(container) = self.api() else {
            return;
        };

        log::info!("Container <{}>: {name}…'", self.id());

        rt::Promise::new(async move { fut_op(container).await }).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |result| {
                match &result {
                    Ok(_) => {
                        log::info!("Container <{}>: {name} has finished", obj.id());
                    }
                    Err(e) => {
                        log::error!("Container <{}>: Error while {name}: {e}", obj.id(),);
                    }
                }
                res_op(result)
            }
        ));
    }
}
