use futures::TryFutureExt;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use podman_api::opts::ImageListOpts;

use crate::model::Image;
use crate::utils::do_async;
use crate::PODMAN;

mod imp {
    use std::cell::{Cell, RefCell};

    use indexmap::IndexMap;
    use once_cell::sync::Lazy;

    use super::*;

    #[derive(Debug, Default)]
    pub struct ImageList {
        pub fetched: Cell<u32>,
        pub list: RefCell<IndexMap<String, Image>>,
        pub to_fetch: Cell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageList {
        const NAME: &'static str = "ImageList";
        type Type = super::ImageList;
        type ParentType = glib::Object;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for ImageList {
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
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "fetched" => obj.set_fetched(value.get().unwrap()),
                "to-fetch" => obj.set_to_fetch(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "fetched" => obj.fetched().to_value(),
                "len" => obj.len().to_value(),
                "to-fetch" => obj.to_fetch().to_value(),
                _ => unimplemented!(),
            }
        }
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.connect_items_changed(|self_, _, _, _| self_.notify("len"));
            obj.fetch_images();
        }
    }

    impl ListModelImpl for ImageList {
        fn item_type(&self, _list_model: &Self::Type) -> glib::Type {
            Image::static_type()
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
    pub struct ImageList(ObjectSubclass<imp::ImageList>)
        @implements gio::ListModel;
}

impl Default for ImageList {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create ImageList")
    }
}

impl ImageList {
    pub fn fetched(&self) -> u32 {
        self.imp().fetched.get()
    }

    fn set_fetched(&self, value: u32) {
        if self.fetched() == value {
            return;
        }
        self.imp().fetched.set(value);
        self.notify("fetched");
    }

    pub fn len(&self) -> u32 {
        self.n_items()
    }

    pub fn to_fetch(&self) -> u32 {
        self.imp().to_fetch.get()
    }

    fn set_to_fetch(&self, value: u32) {
        if self.to_fetch() == value {
            return;
        }
        self.imp().to_fetch.set(value);
        self.notify("to-fetch");
    }

    pub fn total_size(&self) -> u64 {
        self.imp().list.borrow().values().map(Image::size).sum()
    }

    pub fn num_unused_images(&self) -> usize {
        self.imp()
            .list
            .borrow()
            .values()
            .filter(|image| image.repo_tags().is_empty())
            .count()
    }

    pub fn unused_size(&self) -> u64 {
        self.imp()
            .list
            .borrow()
            .values()
            .filter(|image| image.repo_tags().is_empty())
            .map(Image::size)
            .sum()
    }

    pub fn remove_image(&self, id: &str) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, ..)) = list.shift_remove_full(id) {
            drop(list);
            self.items_changed(idx as u32, 1, 0);
        }
    }

    fn fetch_images(&self) {
        do_async(
            glib::PRIORITY_DEFAULT_IDLE,
            async move {
                PODMAN
                    .images()
                    .list(&ImageListOpts::builder().all(true).build())
                    .await
            },
            clone!(@weak self as obj => move |result| async move {
                match result {
                    Ok(summaries) => {
                        {
                            obj.set_to_fetch(summaries.len() as u32);
                            summaries.into_iter().for_each(|summary| {
                                do_async(
                                    glib::PRIORITY_DEFAULT_IDLE,
                                    async move {
                                        PODMAN.images().get(summary.id.as_deref().unwrap()).inspect()
                                            .map_ok(|inspect_response| (summary, inspect_response)).await
                                    },
                                    clone!(@weak obj => move |result| async move {
                                        match result {
                                            Ok((summary, inspect_response)) => {
                                                obj.imp().list.borrow_mut().insert(
                                                    summary.id.clone().unwrap(),
                                                    Image::from_libpod(summary, inspect_response)
                                                );

                                                obj.set_fetched(obj.fetched() + 1);
                                                obj.items_changed(obj.len() - 1, 0, 1);
                                            }
                                            Err(e) => {
                                                log::error!("Error on inspecting image: {}", e);
                                            }
                                        }
                                    })
                                );
                            });
                        }
                    }
                    Err(e) => {
                        log::error!("Error on retrieving images: {}", e);
                    }
                }
            }),
        );
    }
}
