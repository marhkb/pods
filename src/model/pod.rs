use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;

use futures::Future;
use glib::Properties;
use glib::clone;
use glib::prelude::*;
use glib::property::PropertySet;
use glib::subclass::Signal;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;
use crate::model;
use crate::model::prelude::*;
use crate::rt;

mod imp {
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::Pod)]
    pub(crate) struct Pod {
        #[property(get, set, construct_only, nullable)]
        pub(super) pod_list: glib::WeakRef<model::PodList>,
        #[property(get = Self::container_list)]
        pub(super) container_list: OnceCell<model::SimpleContainerList>,
        #[property(get)]
        pub(super) infra_container: glib::WeakRef<model::Container>,

        #[property(get, set, construct_only)]
        pub(super) created: OnceCell<i64>,
        #[property(get, set, construct_only)]
        pub(super) id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) name: OnceCell<String>,
        #[property(get, set, construct, default)]
        pub(super) status: Cell<model::PodStatus>,

        #[property(get = Self::details, set, nullable)]
        pub(super) details: OnceCell<Option<model::PodDetails>>,

        #[property(get, set)]
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
            let handler_id = obj.container_list().connect_container_added(clone!(
                #[weak]
                obj,
                #[strong]
                handler_id_ref,
                move |list, container| if container.is_infra() {
                    list.disconnect(handler_id_ref.take().unwrap());

                    obj.imp().infra_container.set(Some(container));
                    obj.notify_infra_container();
                }
            ));
            handler_id_ref.set(Some(handler_id));
        }
    }

    impl Pod {
        pub(super) fn container_list(&self) -> model::SimpleContainerList {
            self.container_list.get_or_init(Default::default).to_owned()
        }

        pub(super) fn details(&self) -> Option<model::PodDetails> {
            self.details.get().cloned().flatten()
        }
    }
}

glib::wrapper! {
    pub(crate) struct Pod(ObjectSubclass<imp::Pod>) @implements model::Selectable;
}

impl Pod {
    pub(crate) fn new(pod_list: &model::PodList, dto: engine::dto::Pod) -> Self {
        match dto {
            engine::dto::Pod::Summary(dto) => Self::new_from_summary(pod_list, dto),
            engine::dto::Pod::Inspection(dto) => Self::new_from_inspection(pod_list, dto),
        }
    }

    pub(crate) fn new_from_summary(
        pod_list: &model::PodList,
        dto: engine::dto::PodSummary,
    ) -> Self {
        Self::build(pod_list, dto, |builder| builder)
    }

    pub(crate) fn new_from_inspection(
        pod_list: &model::PodList,
        dto: engine::dto::PodInspection,
    ) -> Self {
        Self::build(pod_list, dto.summary, |builder| {
            builder.property("details", Some(model::PodDetails::from(dto.details)))
        })
    }

    fn build<F>(pod_list: &model::PodList, dto: engine::dto::PodSummary, op: F) -> Self
    where
        F: FnOnce(glib::object::ObjectBuilder<Self>) -> glib::object::ObjectBuilder<Self>,
    {
        op(glib::Object::builder()
            .property("pod-list", pod_list)
            .property("created", dto.created)
            .property("id", dto.id)
            .property("name", dto.name)
            .property("status", model::PodStatus::from(dto.status)))
        .build()
    }

    pub(crate) fn update(&self, dto: engine::dto::Pod) {
        match dto {
            engine::dto::Pod::Summary(dto) => self.update_from_summary(dto),
            engine::dto::Pod::Inspection(dto) => self.update_from_inspection(dto),
        }
    }

    pub(crate) fn update_from_summary(&self, dto: engine::dto::PodSummary) {
        self.set_status(model::PodStatus::from(dto.status));
    }

    pub(crate) fn update_from_inspection(&self, dto: engine::dto::PodInspection) {
        self.update_from_summary(dto.summary);

        match self.details() {
            Some(details) => details.update(dto.details),
            None => self.set_details(Some(model::PodDetails::from(dto.details))),
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

    pub(crate) fn api(&self) -> Option<engine::api::Pod> {
        self.pod_list()
            .and_then(|pod_list| pod_list.api())
            .map(|api| api.get(self.id()))
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
    pub(crate) fn start<F>(&self, op: F)
    where
        F: FnOnce(anyhow::Result<()>) + 'static,
    {
        let prev_status = self.status();
        self.set_status(model::PodStatus::Starting);

        self.action(
            "starting",
            |pod| async move { pod.start().await },
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
        self.set_status(model::PodStatus::Stopping);

        self.action(
            if force { "force stopping" } else { "stopping" },
            move |pod| async move { pod.stop(force).await },
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
        self.set_status(model::PodStatus::Restarting);

        self.action(
            if force {
                "force restarting"
            } else {
                "restarting"
            },
            move |pod| async move { pod.restart(force).await },
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
        self.action("pausing", |pod| async move { pod.pause().await }, op);
    }

    pub(crate) fn resume<F>(&self, op: F)
    where
        F: FnOnce(anyhow::Result<()>) + 'static,
    {
        self.action("resuming", |pod| async move { pod.unpause().await }, op);
    }

    pub(crate) fn delete<F>(&self, force: bool, op: F)
    where
        F: FnOnce(anyhow::Result<()>) + 'static,
    {
        self.set_to_be_deleted(true);

        self.action(
            if force { "force deleting" } else { "deleting" },
            move |pod| async move { pod.remove(force).await },
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
        FutOp: FnOnce(engine::api::Pod) -> Fut + Send + 'static,
        ResOp: FnOnce(anyhow::Result<()>) + 'static,
    {
        let Some(pod) = self.api() else { return };

        log::info!("Pod <{}>: {name}…'", self.id());

        rt::Promise::new(async move { fut_op(pod).await }).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |result| {
                match &result {
                    Ok(_) => {
                        log::info!("Pod <{}>: {name} has finished", obj.id());
                    }
                    Err(e) => {
                        log::error!("Pod <{}>: Error while {name}: {e}", obj.id(),);
                    }
                }
                res_op(result)
            }
        ));
    }
}
