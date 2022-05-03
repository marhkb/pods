use std::borrow::Borrow;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashSet;

use futures::TryFutureExt;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::subclass::Signal;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use indexmap::IndexMap;
use once_cell::sync::Lazy;

use crate::api;
use crate::model;
use crate::utils;
use crate::PODMAN;

#[derive(Clone, Debug)]
pub(crate) enum Error {
    List,
    Inspect(String),
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct ImageList {
        pub(super) client: WeakRef<model::Client>,

        pub(super) fetched: Cell<u32>,
        pub(super) list: RefCell<IndexMap<String, model::Image>>,
        pub(super) listing: Cell<bool>,
        pub(super) pruning: Cell<bool>,
        pub(super) to_fetch: Cell<u32>,

        pub(super) failed: RefCell<HashSet<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageList {
        const NAME: &'static str = "ImageList";
        type Type = super::ImageList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for ImageList {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder(
                    "image-added",
                    &[model::Image::static_type().into()],
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
                        "Wether images are currently listed",
                        false,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecBoolean::new(
                        "pruning",
                        "Pruning",
                        "Wether images are currently pruned",
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
                "pruning" => obj.pruning().to_value(),
                "to-fetch" => obj.to_fetch().to_value(),
                _ => unimplemented!(),
            }
        }
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.connect_items_changed(|self_, _, _, _| self_.notify("len"));
        }
    }

    impl ListModelImpl for ImageList {
        fn item_type(&self, _list_model: &Self::Type) -> glib::Type {
            model::Image::static_type()
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
    pub(crate) struct ImageList(ObjectSubclass<imp::ImageList>)
        @implements gio::ListModel;
}

impl From<&model::Client> for ImageList {
    fn from(client: &model::Client) -> Self {
        glib::Object::new(&[("client", client)]).expect("Failed to create ImageList")
    }
}

impl ImageList {
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

    pub(crate) fn pruning(&self) -> bool {
        self.imp().pruning.get()
    }

    fn set_pruning(&self, value: bool) {
        if self.pruning() == value {
            return;
        }
        self.imp().pruning.set(value);
        self.notify("pruning");
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

    pub(crate) fn total_size(&self) -> u64 {
        self.imp()
            .list
            .borrow()
            .values()
            .map(model::Image::size)
            .sum()
    }

    pub(crate) fn num_unused_images(&self) -> usize {
        self.imp()
            .list
            .borrow()
            .values()
            .filter(|image| image.repo_tags().is_empty())
            .count()
    }

    pub(crate) fn unused_size(&self) -> u64 {
        self.imp()
            .list
            .borrow()
            .values()
            .filter(|image| image.repo_tags().is_empty())
            .map(model::Image::size)
            .sum()
    }

    pub(crate) fn get_image<Q: Borrow<str> + ?Sized>(&self, id: &Q) -> Option<model::Image> {
        self.imp().list.borrow().get(id.borrow()).cloned()
    }

    pub(crate) fn remove_image(&self, id: &str) {
        self.imp().failed.borrow_mut().remove(id);

        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, ..)) = list.shift_remove_full(id) {
            drop(list);
            self.items_changed(idx as u32, 1, 0);
        }
    }

    pub(crate) fn prune<F>(&self, opts: api::ImagePruneOpts, op: F)
    where
        F: FnOnce(api::Result<Option<Vec<api::PruneReport>>>) + 'static,
    {
        self.set_pruning(true);
        utils::do_async(
            async move { PODMAN.images().prune(&opts).await },
            clone!(@weak self as obj => move |result| {
                match result.as_ref() {
                    Ok(_) => log::info!("All images have been pruned"),
                    Err(e) => log::error!("Error on pruning images: {e}"),
                }
                obj.set_pruning(false);
                op(result);
            }),
        );
    }

    pub(crate) fn refresh<F>(&self, err_op: F)
    where
        F: FnOnce(Error) + Clone + 'static,
    {
        self.set_listing(true);
        utils::do_async(
            async move {
                PODMAN
                    .images()
                    .list(&api::ImageListOpts::builder().all(true).build())
                    .await
            },
            clone!(@weak self as obj => move |result| {
                obj.set_listing(false);
                match result {
                    Ok(mut summaries) => {
                        let to_remove = obj
                            .imp()
                            .list
                            .borrow()
                            .keys()
                            .filter(|id| {
                                !summaries
                                    .iter()
                                    .any(|summary| summary.id.as_ref() == Some(id))
                            })
                            .cloned()
                            .collect::<Vec<_>>();
                        to_remove.iter().for_each(|id| obj.remove_image(id));

                        summaries.retain(|summary| {
                            !obj.imp().list.borrow().contains_key(summary.id.as_ref().unwrap())
                        });

                        obj.set_fetched(0);
                        obj.set_to_fetch(summaries.len() as u32);

                        summaries.into_iter().for_each(|summary| {
                            utils::do_async(
                                async move {
                                    (
                                        summary.id.clone().unwrap(),
                                        PODMAN
                                            .images()
                                            .get(summary.id.as_deref().unwrap())
                                            .inspect()
                                            .map_ok(|inspect_response| (summary, inspect_response))
                                            .await,
                                    )
                                },
                                clone!(@weak obj, @strong err_op => move |(id, result)| {
                                    let imp = obj.imp();
                                    match result {
                                        Ok((summary, inspect_response)) => {
                                            let image = model::Image::from_libpod(
                                                &obj,
                                                summary,
                                                inspect_response
                                            );
                                            imp.list.borrow_mut().insert(
                                                image.id().to_owned(),
                                                image.clone()
                                            );

                                            obj.image_added(&image);
                                            obj.items_changed(obj.len() - 1, 0, 1);
                                        }
                                        Err(e) => {
                                            log::error!("Error on inspecting image '{id}': {e}");
                                            if imp.failed.borrow_mut().insert(id.clone()) {
                                                err_op(Error::Inspect(id));
                                            }
                                        }
                                    }
                                    obj.set_fetched(obj.fetched() + 1);
                                })
                            );
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

    pub(crate) fn handle_event<F>(&self, event: api::Event, err_op: F)
    where
        F: FnOnce(Error) + Clone + 'static,
    {
        match event.action.as_str() {
            "remove" => self.remove_image(&event.actor.id),
            "build" | "pull" => self.refresh(err_op),
            other => log::warn!("Unknown action: {other}"),
        }
    }

    fn image_added(&self, image: &model::Image) {
        self.emit_by_name::<()>("image-added", &[image]);
    }

    pub(crate) fn connect_image_added<F: Fn(&Self, &model::Image) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("image-added", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let image = values[1].get::<model::Image>().unwrap();
            f(&obj, &image);

            None
        })
    }
}
