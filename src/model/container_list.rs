use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::sync::OnceLock;

use anyhow::anyhow;
use futures::StreamExt;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use indexmap::map::Entry;
use indexmap::map::IndexMap;

use crate::model;
use crate::model::AbstractContainerListExt;
use crate::model::SelectableListExt;
use crate::podman;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ContainerList)]
    pub(crate) struct ContainerList {
        pub(super) list: RefCell<IndexMap<String, model::Container>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get)]
        pub(super) listing: Cell<bool>,
        #[property(get = Self::is_initialized, type = bool)]
        pub(super) initialized: OnceCell<()>,
        #[property(get, set)]
        pub(super) selection_mode: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerList {
        const NAME: &'static str = "ContainerList";
        type Type = super::ContainerList;
        type Interfaces = (
            gio::ListModel,
            model::AbstractContainerList,
            model::SelectableList,
        );
    }

    impl ObjectImpl for ContainerList {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                Self::derived_properties()
                    .iter()
                    .cloned()
                    .chain(vec![
                        glib::ParamSpecUInt::builder("len").read_only().build(),
                        glib::ParamSpecUInt::builder("created").read_only().build(),
                        glib::ParamSpecUInt::builder("dead").read_only().build(),
                        glib::ParamSpecUInt::builder("exited").read_only().build(),
                        glib::ParamSpecUInt::builder("not-running")
                            .read_only()
                            .build(),
                        glib::ParamSpecUInt::builder("paused").read_only().build(),
                        glib::ParamSpecUInt::builder("removing").read_only().build(),
                        glib::ParamSpecUInt::builder("running").read_only().build(),
                        glib::ParamSpecUInt::builder("stopped").read_only().build(),
                        glib::ParamSpecUInt::builder("stopping").read_only().build(),
                        glib::ParamSpecUInt::builder("num-selected")
                            .read_only()
                            .build(),
                    ])
                    .collect::<Vec<_>>()
            })
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "len" => obj.len().to_value(),
                "created" => obj.created().to_value(),
                "dead" => obj.dead().to_value(),
                "exited" => obj.exited().to_value(),
                "not-running" => obj.not_running().to_value(),
                "paused" => obj.paused().to_value(),
                "removing" => obj.removing().to_value(),
                "running" => obj.running().to_value(),
                "stopped" => obj.stopped().to_value(),
                "stopping" => obj.stopping().to_value(),
                "num-selected" => obj.num_selected().to_value(),
                _ => self.derived_property(id, pspec),
            }
        }
        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            model::AbstractContainerList::bootstrap(obj.upcast_ref());
            model::SelectableList::bootstrap(obj.upcast_ref());

            utils::run_stream(
                obj.client().unwrap().podman().containers(),
                |containers| {
                    containers
                        .stats_stream(
                            &podman::opts::ContainerStatsOptsBuilder::default()
                                .interval(1)
                                .build(),
                        )
                        .boxed()
                },
                clone!(
                    @weak obj => @default-return glib::ControlFlow::Break,
                    move |result: podman::Result<podman::models::ContainerStats200Response>|
                {
                    match result
                        .map_err(anyhow::Error::from)
                        .and_then(|mut value| {
                            value
                                .as_object_mut()
                                .and_then(|object| object.remove("Stats"))
                                .ok_or_else(|| anyhow!("Field 'Stats' is not present"))
                        })
                        .and_then(|value| {
                            serde_json::from_value::<Vec<podman::models::ContainerStats>>(value)
                                .map_err(anyhow::Error::from)
                        }) {
                        Ok(stats) => {
                            stats.into_iter().for_each(|stat| {
                                if let Some(container) =
                                    obj.get_container(stat.container_id.as_ref().unwrap())
                                {
                                    if container.status() == model::ContainerStatus::Running {
                                        container.set_stats(
                                            Some(model::BoxedContainerStats::from(stat))
                                        );
                                    }
                                }
                            });
                        }
                        Err(e) => log::warn!("Error occurred on receiving stats stream element: {e}"),
                    }

                    glib::ControlFlow::Continue
                }),
            );

            obj.client().unwrap().podman();
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
        pub(super) fn is_initialized(&self) -> bool {
            self.initialized.get().is_some()
        }

        pub(super) fn set_as_initialized(&self) {
            if self.is_initialized() {
                return;
            }
            self.initialized.set(()).unwrap();
            self.obj().notify_initialized();
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
    pub(crate) fn len(&self) -> u32 {
        self.n_items()
    }

    pub(crate) fn created(&self) -> u32 {
        self.num_containers_of_status(model::ContainerStatus::Created)
    }

    pub(crate) fn dead(&self) -> u32 {
        self.num_containers_of_status(model::ContainerStatus::Dead)
    }

    pub(crate) fn exited(&self) -> u32 {
        self.num_containers_of_status(model::ContainerStatus::Exited)
    }

    pub(crate) fn paused(&self) -> u32 {
        self.num_containers_of_status(model::ContainerStatus::Paused)
    }

    pub(crate) fn removing(&self) -> u32 {
        self.num_containers_of_status(model::ContainerStatus::Removing)
    }

    pub(crate) fn running(&self) -> u32 {
        self.num_containers_of_status(model::ContainerStatus::Running)
    }

    pub(crate) fn stopped(&self) -> u32 {
        self.num_containers_of_status(model::ContainerStatus::Stopped)
    }

    pub(crate) fn stopping(&self) -> u32 {
        self.num_containers_of_status(model::ContainerStatus::Stopping)
    }

    pub(crate) fn num_containers_of_status(&self, status: model::ContainerStatus) -> u32 {
        self.imp()
            .list
            .borrow()
            .values()
            .filter(|container| container.status() == status)
            .count() as u32
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

    pub(crate) fn refresh<F>(&self, id: Option<String>, err_op: F)
    where
        F: FnOnce(super::RefreshError) + Clone + 'static,
    {
        self.imp().set_listing(true);
        utils::do_async(
            {
                let podman = self.client().unwrap().podman();
                let id = id.clone();
                async move {
                    podman
                        .containers()
                        .list(
                            &podman::opts::ContainerListOpts::builder()
                                .all(true)
                                .filter(
                                    id.map(podman::Id::from)
                                        .map(podman::opts::ContainerListFilter::Id),
                                )
                                .build(),
                        )
                        .await
                }
            },
            clone!(@weak self as obj => move |result| {
                match result {
                    Ok(list_containers) => {
                        if id.is_none() {
                            let to_remove = obj
                                .imp()
                                .list
                                .borrow()
                                .keys()
                                .filter(|id| {
                                    !list_containers
                                        .iter()
                                        .any(|list_container| list_container.id.as_ref() == Some(id))
                                })
                                .cloned()
                                .collect::<Vec<_>>();
                            to_remove.iter().for_each(|id| {
                                obj.remove_container(id);
                            });
                        }

                        list_containers
                            .into_iter()
                            .filter(|list_container| !list_container.is_infra.unwrap_or_default())
                            .for_each(|list_container| {
                                let index = obj.len();

                                let mut list = obj.imp().list.borrow_mut();

                                match list.entry(list_container.id.as_ref().unwrap().to_owned()) {
                                    Entry::Vacant(e) => {
                                        let container = model::Container::new(&obj, list_container);
                                        e.insert(container.clone());

                                        drop(list);

                                        obj.items_changed(index, 0, 1);
                                        obj.container_added(&container);
                                    }
                                    Entry::Occupied(e) => {
                                        let container = e.get().clone();
                                        drop(list);
                                        container.update(list_container);
                                    }
                                }
                            });
                        }
                    Err(e) => {
                        log::error!("Error on retrieving containers: {}", e);
                        err_op(super::RefreshError);
                    }
                }
                let imp = obj.imp();
                imp.set_listing(false);
                imp.set_as_initialized();
            }),
        );
    }

    pub(crate) fn handle_event<F>(&self, event: podman::models::Event, err_op: F)
    where
        F: FnOnce(super::RefreshError) + Clone + 'static,
    {
        let container_id = event.actor.id;

        match event.action.as_str() {
            "remove" => self.remove_container(&container_id),
            "health_status" => {
                if let Some(container) = self.get_container(&container_id) {
                    container.inspect(|_| {});
                }
            }
            _ => self.refresh(
                self.get_container(&container_id).map(|_| container_id),
                err_op,
            ),
        }
    }
}
