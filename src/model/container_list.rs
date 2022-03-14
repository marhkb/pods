use std::cell::{Cell, RefCell};

use gtk::glib::clone;
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
        pub(super) fetched: Cell<u32>,
        pub(super) list: RefCell<IndexMap<String, model::Container>>,
        pub(super) listing: Cell<bool>,
        pub(super) to_fetch: Cell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerList {
        const NAME: &'static str = "ContainerList";
        type Type = super::ContainerList;
        type ParentType = glib::Object;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for ContainerList {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
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

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
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

impl Default for ContainerList {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create ContainerList")
    }
}

impl ContainerList {
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
        let err = Error::Inspect(id.clone());
        utils::do_async(
            async move { PODMAN.containers().get(id).inspect().await },
            clone!(@weak self as obj => move |result| {
                obj.set_fetched(obj.fetched() + 1);
                match result {
                    Ok(inspect_response) => {
                        let mut list = obj.imp().list.borrow_mut();
                        let entry = list.entry(inspect_response.id.clone().unwrap());
                        match entry {
                            Entry::Vacant(entry) => {
                                entry.insert(model::Container::from(inspect_response));

                                drop(list);
                                obj.items_changed(obj.len() - 1, 0, 1);
                            }
                            Entry::Occupied(entry) => {
                                entry.get().update(inspect_response);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error on inspecting image: {}", e);
                        err_op(err);
                    }
                }
            }),
        );
    }

    pub(crate) fn remove_container(&self, id: &str) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, ..)) = list.shift_remove_full(id) {
            drop(list);
            self.items_changed(idx as u32, 1, 0);
        }
    }

    fn refresh<F>(&self, err_op: F)
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

    pub(crate) fn setup<F>(&self, err_op: F)
    where
        F: FnOnce(Error) + Clone + 'static,
    {
        utils::run_stream(
            PODMAN.events(
                &api::EventsOpts::builder()
                    .filters([("type".to_string(), vec!["container".to_string()])])
                    .build(),
            ),
            clone!(
                @weak self as obj, @strong err_op=> @default-return glib::Continue(false),
                move |result|
            {
                glib::Continue(match result {
                    Ok(event) => {
                        log::debug!("Event: {event:?}");
                        obj.refresh(err_op.clone());
                        true
                    },
                    Err(e) => {
                        log::error!("Stopping image event stream due to error: {e}");
                        false
                    }
                })
            }),
        );

        self.refresh(err_op);
    }
}
