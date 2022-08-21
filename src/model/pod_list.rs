use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashSet;

use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::subclass::Signal;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use indexmap::map::Entry;
use indexmap::map::IndexMap;
use once_cell::sync::Lazy;

use crate::model;
use crate::podman;
use crate::utils;

mod imp {

    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct PodList {
        pub(super) client: WeakRef<model::Client>,

        pub(super) fetched: Cell<u32>,
        pub(super) list: RefCell<IndexMap<String, model::Pod>>,
        pub(super) listing: Cell<bool>,
        pub(super) to_fetch: Cell<u32>,

        pub(super) failed: RefCell<HashSet<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodList {
        const NAME: &'static str = "PodList";
        type Type = super::PodList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for PodList {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder(
                    "pod-added",
                    &[model::Pod::static_type().into()],
                    <()>::static_type().into(),
                )
                .build()]
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
                        "The number of pods that have been fetched",
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
                        "Wether pods are currently listed",
                        false,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt::new(
                        "running",
                        "Running",
                        "The number of running pods",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt::new(
                        "to-fetch",
                        "To Fetch",
                        "The number of pods to be fetched",
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
                "running" => obj.running().to_value(),
                "to-fetch" => obj.to_fetch().to_value(),
                _ => unimplemented!(),
            }
        }
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.connect_items_changed(|self_, _, _, _| self_.notify("len"));
        }
    }

    impl ListModelImpl for PodList {
        fn item_type(&self, _list_model: &Self::Type) -> glib::Type {
            model::Pod::static_type()
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
    pub(crate) struct PodList(ObjectSubclass<imp::PodList>) @implements gio::ListModel;
}

impl From<Option<&model::Client>> for PodList {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::new(&[("client", &client)]).expect("Failed to create PodList")
    }
}

impl PodList {
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

    pub(crate) fn running(&self) -> u32 {
        self.imp()
            .list
            .borrow()
            .values()
            .filter(|pod| pod.status() == model::PodStatus::Running)
            .count() as u32
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

    fn add_or_update_pod<F>(&self, id: String, err_op: F)
    where
        F: FnOnce(super::RefreshError) + 'static,
    {
        utils::do_async(
            {
                let id = id.clone();
                let podman = self.client().unwrap().podman().clone();
                async move { podman.pods().get(id).inspect().await }
            },
            clone!(@weak self as obj => move |result| {
                let imp = obj.imp();
                match result {
                    Ok(inspect_response) => {
                        let mut list = imp.list.borrow_mut();
                        let entry = list.entry(id);
                        match entry {
                            Entry::Vacant(entry) => {
                                let pod = model::Pod::new(&obj, inspect_response);
                                pod.connect_notify_local(
                                    Some("status"),
                                    clone!(@weak obj => move |_, _| {
                                        obj.notify("running");
                                    }),
                                );
                                entry.insert(pod.clone());

                                drop(list);
                                obj.pod_added(&pod);
                                obj.items_changed(obj.len() - 1, 0, 1);
                            }
                            Entry::Occupied(entry) => {
                                let pod = entry.get().clone();
                                drop(list);
                                pod.update(inspect_response)
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error on inspecting pod '{id}': {e}");
                        if imp.failed.borrow_mut().insert(id.clone()) {
                            err_op(super::RefreshError::Inspect(id));
                        }
                    }
                }
                obj.set_fetched(obj.fetched() + 1);
            }),
        );
    }

    pub(crate) fn get_pod(&self, id: &str) -> Option<model::Pod> {
        self.imp().list.borrow().get(id).cloned()
    }

    pub(crate) fn remove_pod(&self, id: &str) {
        self.imp().failed.borrow_mut().remove(id);

        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _, pod)) = list.shift_remove_full(id) {
            pod.emit_deleted();
            drop(list);
            self.items_changed(idx as u32, 1, 0);
        }
    }

    pub(crate) fn refresh<F>(&self, err_op: F)
    where
        F: FnOnce(super::RefreshError) + Clone + 'static,
    {
        self.set_listing(true);
        utils::do_async(
            {
                let podman = self.client().unwrap().podman().clone();
                async move {
                    podman
                        .pods()
                        .list(&podman::opts::PodListOpts::builder().build())
                        .await
                }
            },
            clone!(@weak self as obj => move |result| {
                obj.set_listing(false);
                match result {
                    Ok(list_pods) => {
                        let to_remove = obj
                            .imp()
                            .list
                            .borrow()
                            .keys()
                            .filter(|id| {
                                !list_pods
                                    .iter()
                                    .any(|summary| summary.id.as_ref() == Some(id))
                            })
                            .cloned()
                            .collect::<Vec<_>>();
                        to_remove.iter().for_each(|id| obj.remove_pod(id));

                        obj.set_fetched(0);
                        obj.set_to_fetch(list_pods.len() as u32);

                        list_pods.into_iter().for_each(|list_container| {
                            obj.add_or_update_pod(list_container.id.unwrap(), err_op.clone());
                        });
                    }
                    Err(e) => {
                        log::error!("Error on retrieving pods: {}", e);
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
        let pod_id = event.actor.id;

        match event.action.as_str() {
            "remove" => self.remove_pod(&pod_id),
            _ => self.add_or_update_pod(pod_id, err_op),
        }
    }

    fn pod_added(&self, pod: &model::Pod) {
        self.emit_by_name::<()>("pod-added", &[pod]);
    }

    pub(crate) fn connect_pod_added<F: Fn(&Self, &model::Pod) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("pod-added", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let pod = values[1].get::<model::Pod>().unwrap();
            f(&obj, &pod);

            None
        })
    }
}
