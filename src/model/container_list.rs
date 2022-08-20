use std::cell::Cell;
use std::cell::RefCell;

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

use crate::api;
use crate::model;
use crate::model::AbstractContainerListExt;
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
                "running" => obj.running().to_value(),
                _ => unimplemented!(),
            }
        }
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.connect_items_changed(|self_, _, _, _| self_.notify("len"));

            utils::run_stream(
                obj.client().unwrap().podman().containers(),
                |containers| {
                    containers
                        .stats_stream(
                            &api::ContainerStatsOptsBuilder::default()
                                .interval(1)
                                .build(),
                        )
                        .boxed()
                },
                clone!(
                    @weak obj => @default-return glib::Continue(false),
                    move |result: api::Result<api::LibpodContainerStatsResponse>|
                {
                    glib::Continue(match result.ok().and_then(|stats| stats.stats) {
                        Some(stats) => {
                            stats.into_iter().for_each(|stat| {
                                if let Some(container) = obj.get_container(stat.container_id.as_ref().unwrap()) {
                                    if container.status() == model::ContainerStatus::Running {
                                        container.set_stats(Some(model::BoxedContainerStats::from(stat)));
                                    }
                                }
                            });
                            true
                        }
                        None => false,
                    })
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

    pub(crate) fn running(&self) -> u32 {
        self.imp()
            .list
            .borrow()
            .values()
            .filter(|container| container.status() == model::ContainerStatus::Running)
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
                            &api::ContainerListOpts::builder()
                                .all(true)
                                .filter(
                                    id.to_owned()
                                        .map(api::Id::from)
                                        .map(api::ContainerListFilter::Id),
                                )
                                .build(),
                        )
                        .await
                }
            },
            clone!(@weak self as obj => move |result| {
                obj.set_listing(false);
                match result {
                    Ok(list_containers) => {
                        let index = obj.len();
                        let mut added = 0;

                        list_containers
                        .into_iter()
                        .filter(|list_container| !list_container.is_infra.unwrap_or_default())
                        .for_each(|list_container| {
                            let mut list = obj.imp().list.borrow_mut();

                            match list.entry(list_container.id.as_ref().unwrap().to_owned()) {
                                Entry::Vacant(e) => {
                                    let container = model::Container::new(&obj, list_container);
                                    container.connect_notify_local(
                                        Some("status"),
                                        clone!(@weak obj => move |_, _| {
                                            obj.notify("running");
                                        }),
                                    );
                                    container.connect_notify_local(
                                        Some("name"),
                                        clone!(@weak obj => move |container, _| {
                                            obj.container_name_changed(container);
                                        })
                                    );

                                    e.insert(container.clone());
                                    obj.container_added(&container);

                                    added += 1;
                                }
                                Entry::Occupied(e) => {
                                    let container = e.get().clone();
                                    drop(list);
                                    container.update(list_container);
                                }
                            }
                        });

                        obj.items_changed(index, 0, added as u32);
                    }
                    Err(e) => {
                        log::error!("Error on retrieving containers: {}", e);
                        err_op(super::RefreshError::List);
                    }
                }
            }),
        );
    }

    pub(crate) fn handle_event<F>(&self, event: api::Event, err_op: F)
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
