use std::cell::{Cell, RefCell};
use std::collections::HashSet;

use gtk::glib::subclass::Signal;
use gtk::glib::{clone, WeakRef};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use indexmap::map::{Entry, IndexMap};
use once_cell::sync::Lazy;

use crate::{api, model, utils, PODMAN};

#[derive(Clone, Debug)]
pub(crate) enum Error {
    List,
    Inspect(String),
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct ContainerList {
        pub(super) client: WeakRef<model::Client>,

        pub(super) fetched: Cell<u32>,
        pub(super) list: RefCell<IndexMap<String, model::Container>>,
        pub(super) listing: Cell<bool>,
        pub(super) to_fetch: Cell<u32>,

        pub(super) failed: RefCell<HashSet<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerList {
        const NAME: &'static str = "ContainerList";
        type Type = super::ContainerList;
        type ParentType = glib::Object;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for ContainerList {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder(
                        "container-added",
                        &[model::Container::static_type().into()],
                        <()>::static_type().into(),
                    )
                    .build(),
                    Signal::builder(
                        "container-removed",
                        &[str::static_type().into()],
                        <()>::static_type().into(),
                    )
                    .build(),
                ]
            });
            SIGNALS.as_ref()
        }

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
                        "fetched",
                        "Fetched",
                        "The number of images that have been fetched",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READABLE,
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
                        "to-fetch",
                        "To Fetch",
                        "The number of images to be fetched",
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
                "fetched" => obj.fetched().to_value(),
                "len" => obj.len().to_value(),
                "listing" => obj.listing().to_value(),
                "to-fetch" => obj.to_fetch().to_value(),
                _ => unimplemented!(),
            }
        }
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.connect_items_changed(|self_, _, _, _| self_.notify("len"));
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
        @implements gio::ListModel;
}

impl From<&model::Client> for ContainerList {
    fn from(client: &model::Client) -> Self {
        glib::Object::new(&[("client", client)]).expect("Failed to create ContainerList")
    }
}

impl ContainerList {
    pub(crate) fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    pub(crate) fn fetched(&self) -> u32 {
        self.imp().fetched.get()
    }

    fn set_fetched(&self, value: u32) {
        if self.fetched() == value {
            return;
        }
        self.imp().fetched.set(value);
        self.notify("fetched");
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

    pub(crate) fn to_fetch(&self) -> u32 {
        self.imp().to_fetch.get()
    }

    fn set_to_fetch(&self, value: u32) {
        if self.to_fetch() == value {
            return;
        }
        self.imp().to_fetch.set(value);
        self.notify("to-fetch");
    }

    pub(crate) fn count(&self, status: model::ContainerStatus) -> usize {
        self.imp()
            .list
            .borrow()
            .values()
            .filter(|container| container.status() == status)
            .count()
    }

    fn add_or_update_container<F>(&self, id: String, err_op: F)
    where
        F: FnOnce(Error) + 'static,
    {
        utils::do_async(
            async move { (PODMAN.containers().get(id.as_str()).inspect().await, id) },
            clone!(@weak self as obj => move |(result, id)| {
                let imp = obj.imp();
                match result {
                    Ok(inspect_response) => {
                        let mut list = imp.list.borrow_mut();
                        let entry = list.entry(id);
                        match entry {
                            Entry::Vacant(entry) => {
                                let container = model::Container::from(inspect_response);
                                entry.insert(container.clone());

                                drop(list);
                                obj.container_added(&container);
                                obj.items_changed(obj.len() - 1, 0, 1);
                            }
                            Entry::Occupied(entry) => {
                                entry.get().update(inspect_response);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error on inspecting container '{id}': {e}");
                        if imp.failed.borrow_mut().insert(id.clone()) {
                            err_op(Error::Inspect(id));
                        }
                    }
                }
                obj.set_fetched(obj.fetched() + 1);
            }),
        );
    }

    pub(crate) fn remove_container(&self, id: &str) {
        self.imp().failed.borrow_mut().remove(id);

        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, id, _)) = list.shift_remove_full(id) {
            drop(list);
            self.container_removed(&id);
            self.items_changed(idx as u32, 1, 0);
        }
    }

    pub(crate) fn refresh<F>(&self, err_op: F)
    where
        F: FnOnce(Error) + Clone + 'static,
    {
        self.set_listing(true);
        utils::do_async(
            async move {
                PODMAN
                    .containers()
                    .list(&api::ContainerListOpts::builder().all(true).build())
                    .await
            },
            clone!(@weak self as obj => move |result| {
                obj.set_listing(false);
                match result {
                    Ok(list_containers) => {
                        let to_remove = obj
                            .imp()
                            .list
                            .borrow()
                            .keys()
                            .filter(|id| {
                                !list_containers
                                    .iter()
                                    .any(|summary| summary.id.as_ref() == Some(id))
                            })
                            .cloned()
                            .collect::<Vec<_>>();
                        to_remove.iter().for_each(|id| obj.remove_container(id));

                        obj.set_fetched(0);
                        obj.set_to_fetch(list_containers.len() as u32);

                        list_containers.into_iter().for_each(|list_container| {
                            obj.add_or_update_container(list_container.id.unwrap(), err_op.clone());
                        });
                    }
                    Err(e) => {
                        log::error!("Error on retrieving images: {}", e);
                        err_op(Error::List);
                    }
                }
            }),
        );
    }

    pub(crate) fn handle_event<F>(&self, _event: api::Event, err_op: F)
    where
        F: FnOnce(Error) + Clone + 'static,
    {
        self.refresh(err_op);
    }

    fn container_added(&self, container: &model::Container) {
        self.emit_by_name::<()>("container-added", &[container]);
    }

    fn container_removed(&self, id: &str) {
        self.emit_by_name::<()>("container-removed", &[&id]);
    }

    pub(crate) fn connect_container_added<F: Fn(&Self, &model::Container) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("container-added", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let container = values[1].get::<model::Container>().unwrap();
            f(&obj, &container);

            None
        })
    }

    pub(crate) fn connect_container_removed<F: Fn(&Self, &str) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("container-removed", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let id = values[1].get::<&str>().unwrap();
            f(&obj, id);

            None
        })
    }
}
