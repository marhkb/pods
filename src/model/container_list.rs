use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::marker::PhantomData;

use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use gtk::gio;
use gtk::glib;
use indexmap::map::IndexMap;

use crate::engine;
use crate::model;
use crate::model::prelude::*;
use crate::rt;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ContainerList)]
    pub(crate) struct ContainerList {
        pub(super) list: RefCell<IndexMap<String, model::Container>>,

        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,

        #[property(get = |imp: &Self| imp.initialized.get().is_some(), type = bool)]
        pub(super) initialized: OnceCell<()>,
        #[property(get)]
        pub(super) listing: Cell<bool>,
        #[property(get, set)]
        pub(super) selection_mode: Cell<bool>,

        #[property(get = |imp: &Self| imp
            .list
            .borrow()
            .values()
            .filter(|container| !container.is_infra())
            .count() as u32
        )]
        _containers: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Created))]
        _created: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Dead))]
        _dead: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Exited))]
        _exited: PhantomData<u32>,
        #[property(get = Self::n_items)]
        _len: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.obj().containers() - imp.obj().running())]
        _not_running: PhantomData<u32>,
        #[property(get = |imp: &Self| model::SelectableListExt::num_selected(&*imp.obj()))]
        _num_selected: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Paused))]
        _paused: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Removing))]
        _removing: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Running))]
        _running: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Stopped))]
        _stopped: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Stopping))]
        _stopping: PhantomData<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerList {
        const NAME: &'static str = "PdsContainerList";
        type Type = super::ContainerList;
        type Interfaces = (
            gio::ListModel,
            model::AbstractContainerList,
            model::SelectableList,
        );
    }

    impl ObjectImpl for ContainerList {
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

            model::AbstractContainerList::bootstrap(obj.upcast_ref());
            model::SelectableList::bootstrap(obj.upcast_ref());

            rt::Pipe::new(obj.client().unwrap().engine().containers(), |containers| {
                containers.stats_stream(1)
            })
            .on_next(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move |all_stats| {
                    match all_stats {
                        Ok(all_stats) => {
                            all_stats
                                .into_single_stats()
                                .into_iter()
                                .for_each(|(id, stats)| {
                                    if let Some(container) = obj.get_container(&id)
                                        && container.status() == model::ContainerStatus::Running
                                    {
                                        container.set_stats(Some(
                                            model::BoxedContainerStats::from(stats),
                                        ));
                                    }
                                });
                        }
                        Err(e) => {
                            log::warn!("Error occurred on receiving stats stream element: {e}")
                        }
                    }

                    glib::ControlFlow::Continue
                }
            ));
        }
    }

    impl ListModelImpl for ContainerList {
        fn item_type(&self) -> glib::Type {
            model::Container::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .map(|(_, obj)| obj.upcast_ref())
                .cloned()
        }
    }

    impl ContainerList {
        fn num_containers_of_status(&self, status: model::ContainerStatus) -> u32 {
            self.list
                .borrow()
                .values()
                .filter(|container| !container.is_infra() && container.status() == status)
                .count() as u32
        }

        pub(super) fn set_as_initialized(&self) {
            if self.initialized.set(()).is_ok() {
                self.obj().notify_initialized();
            }
        }

        pub(super) fn set_listing(&self, value: bool) {
            let obj = &*self.obj();
            if obj.listing() == value {
                return;
            }
            self.listing.set(value);
            obj.notify_listing();
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerList(ObjectSubclass<imp::ContainerList>)
        @implements gio::ListModel, model::AbstractContainerList, model::SelectableList;
}

impl From<&model::Client> for ContainerList {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl ContainerList {
    fn upsert_container(&self, dto: engine::dto::Container) {
        match self.get_container(dto.id()) {
            Some(container) => container.update(dto),
            None => {
                let container = model::Container::new(self, dto);

                let index = self.len();

                self.imp()
                    .list
                    .borrow_mut()
                    .insert(container.id(), container.clone());

                self.items_changed(index, 0, 1);
                self.container_added(&container);
            }
        }
    }

    pub(crate) fn get_container(&self, id: &str) -> Option<model::Container> {
        self.imp().list.borrow().get(id).cloned()
    }

    pub(crate) fn remove_container(&self, id: &str) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _, container)) = list.shift_remove_full(id) {
            drop(list);

            self.items_changed(idx as u32, 1, 0);
            self.container_removed(&container);
            container.on_deleted();
        }
    }

    pub(crate) fn refresh<F>(&self, err_op: F)
    where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        let Some(api) = self.api() else { return };

        self.imp().set_listing(true);

        rt::Promise::new(async move { api.list().await }).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |dtos| {
                match dtos {
                    Ok(dtos) => {
                        let to_remove = obj
                            .imp()
                            .list
                            .borrow()
                            .keys()
                            .filter(|id| !dtos.iter().any(|dto| dto.id() == *id))
                            .cloned()
                            .collect::<Vec<_>>();
                        to_remove.iter().for_each(|id| {
                            obj.remove_container(id);
                        });

                        dtos.into_iter().for_each(|dto| obj.upsert_container(dto));
                    }
                    Err(e) => {
                        log::error!("Error on retrieving containers: {}", e);
                        err_op(e);
                    }
                }
                let imp = obj.imp();
                imp.set_listing(false);
                imp.set_as_initialized();
            }
        ));
    }

    pub(crate) fn api(&self) -> Option<engine::api::Containers> {
        self.client()
            .map(|client| client.engine())
            .map(|engine| engine.containers())
    }
}

// Events
impl ContainerList {
    pub(crate) fn handle_event<F>(&self, event: engine::dto::Event, err_op: F)
    where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        match event {
            engine::Response::Docker(event) => self.handle_docker_event(event, err_op),
            engine::Response::Podman(event) => self.handle_podman_event(event, err_op),
        }
    }

    fn handle_docker_event<F>(&self, event: bollard::plugin::EventMessage, err_op: F)
    where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        let actor = event.actor.unwrap();
        let id = actor.id.unwrap();

        match event.action.as_deref().unwrap() {
            "create" => self.upsert_container_fetch(id, err_op),
            "init" => self.upsert_container_status(id, model::ContainerStatus::Initialized, err_op),
            "start" | "restart" => self.upsert_container_with(
                id,
                |container| {
                    container.set_status(model::ContainerStatus::Running);
                    if let Some(details) = container.details() {
                        details.set_up_since(event.time.unwrap_or_default());
                    }
                },
                err_op,
            ),
            "pause" => self.upsert_container_status(id, model::ContainerStatus::Paused, err_op),
            "unpause" => self.upsert_container_status(id, model::ContainerStatus::Running, err_op),
            "kill" | "stop" => {
                self.upsert_container_status(id, model::ContainerStatus::Stopping, err_op)
            }
            "cleanup" | "die" | "died" => {
                self.upsert_container_status(id, model::ContainerStatus::Exited, err_op)
            }
            "oom" => self.upsert_container_status(id, model::ContainerStatus::Dead, err_op),
            "rename" => self.upsert_container_with(
                id,
                |container| {
                    if let Some(name) = actor.attributes.unwrap().remove("name") {
                        container.set_name(name);
                    }
                },
                err_op,
            ),
            "exec_die" => self.upsert_container_with(
                id,
                clone!(
                    #[weak(rename_to = obj)]
                    self,
                    #[strong]
                    err_op,
                    move |container| obj.upsert_container_fetch(container.id(), err_op.clone(),)
                ),
                err_op,
            ),
            "health_status: healthy" => self.upsert_container_health_status(
                id,
                model::ContainerHealthStatus::Healthy,
                err_op,
            ),
            "health_status: unhealthy" => self.upsert_container_health_status(
                id,
                model::ContainerHealthStatus::Unhealthy,
                err_op,
            ),
            "destroy" => self.remove_container(&id),
            other => log::debug!("unhandled container event type: {other}"),
        }
    }

    fn handle_podman_event<F>(&self, mut event: podman_api::models::Event, err_op: F)
    where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        let id = event.actor.id;

        match event.action.as_str() {
            "create" => self.upsert_container_fetch(id, err_op),
            "init" => self.upsert_container_status(id, model::ContainerStatus::Initialized, err_op),
            "start" => self.upsert_container_with(
                id,
                |container| {
                    container.set_status(model::ContainerStatus::Running);
                    if let Some(details) = container.details() {
                        details.set_up_since(event.time as i64);
                    }
                },
                err_op,
            ),
            "pause" => self.upsert_container_status(id, model::ContainerStatus::Paused, err_op),
            "unpause" => self.upsert_container_status(id, model::ContainerStatus::Running, err_op),
            "kill" | "stop" => {
                self.upsert_container_status(id, model::ContainerStatus::Stopping, err_op)
            }
            "cleanup" | "die" | "died" => {
                self.upsert_container_status(id, model::ContainerStatus::Exited, err_op)
            }
            "restart" => {
                self.upsert_container_status(id, model::ContainerStatus::Restarting, err_op)
            }
            "oom" => self.upsert_container_status(id, model::ContainerStatus::Dead, err_op),
            "rename" => self.upsert_container_with(
                id,
                |container| {
                    if let Some(name) = event.actor.attributes.remove("name") {
                        container.set_name(name);
                    }
                },
                err_op,
            ),
            "health_status" => self.upsert_container_with(
                id,
                clone!(
                    #[weak(rename_to = obj)]
                    self,
                    #[strong]
                    err_op,
                    move |container| obj.upsert_container_fetch(container.id(), err_op.clone(),)
                ),
                err_op,
            ),
            "remove" => self.remove_container(&id),
            other => log::debug!("unhandled container event type: {other}"),
        }
    }

    fn upsert_container_fetch<F>(&self, id: String, err_op: F)
    where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        let Some(api) = self.api().map(|api| api.get(id)) else {
            return;
        };

        rt::Promise::new(async move { api.inspect().await }).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |dto| match dto {
                Ok(dto) => obj.upsert_container(engine::dto::Container::Inspection(dto)),
                Err(e) => err_op(e),
            }
        ));
    }

    fn upsert_container_health_status<F>(
        &self,
        id: String,
        health_status: model::ContainerHealthStatus,
        err_op: F,
    ) where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        log::error!("{id}: {health_status}");
        self.upsert_container_with(
            id,
            |container| {
                container.set_health_status(health_status);
            },
            err_op,
        );
    }

    fn upsert_container_status<F>(&self, id: String, status: model::ContainerStatus, err_op: F)
    where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        self.upsert_container_with(id, |container| container.set_status(status), err_op);
    }

    fn upsert_container_with<F, E>(&self, id: String, op: F, err_op: E)
    where
        F: FnOnce(&model::Container),
        E: FnOnce(anyhow::Error) + Clone + 'static,
    {
        match self.get_container(&id) {
            Some(container) => op(&container),
            None => self.upsert_container_fetch(id.to_owned(), err_op),
        }
    }
}
