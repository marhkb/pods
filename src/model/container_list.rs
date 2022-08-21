use std::cell::Cell;
use std::cell::RefCell;

use anyhow::anyhow;
use futures::StreamExt;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use indexmap::map::Entry;
use indexmap::map::IndexMap;
use once_cell::sync::Lazy;

use crate::model;
use crate::model::AbstractContainerListExt;
use crate::podman;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct ContainerList {
        pub(super) client: WeakRef<model::Client>,
        pub(super) list: RefCell<IndexMap<String, model::Container>>,
        pub(super) listing: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerList {
        const NAME: &'static str = "ContainerList";
        type Type = super::ContainerList;
        type Interfaces = (gio::ListModel, model::AbstractContainerList);
    }

    impl ObjectImpl for ContainerList {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "client",
                        "Client",
                        "The podman client",
                        model::Client::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecUInt::new(
                        "len",
                        "Len",
                        "The length of this list",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecBoolean::new(
                        "listing",
                        "Listing",
                        "Wether containers are currently listed",
                        false,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt::new(
                        "created",
                        "Created",
                        "The number of created containers",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt::new(
                        "dead",
                        "Dead",
                        "The number of dead containers",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt::new(
                        "exited",
                        "Exited",
                        "The number of exited containers",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt::new(
                        "paused",
                        "Paused",
                        "The number of paused containers",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt::new(
                        "running",
                        "Running",
                        "The number of running containers",
                        0,
                        std::u32::MAX,
                        0,
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
                "client" => self.client.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "client" => obj.client().to_value(),
                "len" => obj.len().to_value(),
                "listing" => obj.listing().to_value(),
                "created" => obj.created().to_value(),
                "dead" => obj.dead().to_value(),
                "exited" => obj.exited().to_value(),
                "paused" => obj.paused().to_value(),
                "running" => obj.running().to_value(),
                _ => unimplemented!(),
            }
        }
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            model::AbstractContainerList::bootstrap(obj);

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
                    @weak obj => @default-return glib::Continue(false),
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
                        Err(e) => log::warn!("Error occured on receiving stats stream element: {e}"),
                    }

                    glib::Continue(true)
                }),
            );

            obj.client().unwrap().podman();
        }
    }

    impl ListModelImpl for ContainerList {
        fn item_type(&self, _list_model: &Self::Type) -> glib::Type {
            model::Container::static_type()
        }

        fn n_items(&self, _list_model: &Self::Type) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, _list_model: &Self::Type, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .map(|(_, obj)| obj.upcast_ref())
                .cloned()
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerList(ObjectSubclass<imp::ContainerList>)
        @implements gio::ListModel, model::AbstractContainerList;
}

impl From<Option<&model::Client>> for ContainerList {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::new(&[("client", &client)]).expect("Failed to create ContainerList")
    }
}

impl ContainerList {
    pub(crate) fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    pub(crate) fn len(&self) -> u32 {
        self.n_items()
    }

    pub(crate) fn listing(&self) -> bool {
        self.imp().listing.get()
    }

    fn set_listing(&self, value: bool) {
        if self.listing() == value {
            return;
        }
        self.imp().listing.set(value);
        self.notify("listing");
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

    pub(crate) fn running(&self) -> u32 {
        self.num_containers_of_status(model::ContainerStatus::Running)
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
            container.on_deleted();
            drop(list);
            self.container_removed(&container);
            self.items_changed(idx as u32, 1, 0);
        }
    }

    pub(crate) fn refresh<F>(&self, id: Option<String>, err_op: F)
    where
        F: FnOnce(super::RefreshError) + Clone + 'static,
    {
        self.set_listing(true);
        utils::do_async(
            {
                let podman = self.client().unwrap().podman().clone();
                async move {
                    podman
                        .containers()
                        .list(
                            &podman::opts::ContainerListOpts::builder()
                                .all(true)
                                .filter(
                                    id.to_owned()
                                        .map(podman::Id::from)
                                        .map(podman::opts::ContainerListFilter::Id),
                                )
                                .build(),
                        )
                        .await
                }
            },
            clone!(@weak self as obj => move |result| {
                obj.set_listing(false);
                match result {
                    Ok(list_containers) => list_containers
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
                        }),
                    Err(e) => {
                        log::error!("Error on retrieving containers: {}", e);
                        err_op(super::RefreshError::List);
                    }
                }
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
            _ => self.refresh(
                self.get_container(&container_id).map(|_| container_id),
                err_op,
            ),
        }
    }
}
