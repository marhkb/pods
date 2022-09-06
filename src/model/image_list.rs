use std::borrow::Borrow;
use std::cell::Cell;
use std::cell::RefCell;

use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::subclass::Signal;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use indexmap::map::Entry;
use indexmap::IndexMap;
use once_cell::sync::Lazy;

use crate::model;
use crate::model::SelectableListExt;
use crate::podman;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct ImageList {
        pub(super) client: WeakRef<model::Client>,
        pub(super) list: RefCell<IndexMap<String, model::Image>>,
        pub(super) listing: Cell<bool>,
        pub(super) selection_mode: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageList {
        const NAME: &'static str = "ImageList";
        type Type = super::ImageList;
        type Interfaces = (gio::ListModel, model::SelectableList);
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
                        "selection-mode",
                        "Selection Mode",
                        "Wether the selection mode is active",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpecUInt::new(
                        "num-selected",
                        "Num Selected",
                        "The number of selected images",
                        0,
                        u32::MAX,
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
                "selection-mode" => self.selection_mode.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "client" => obj.client().to_value(),
                "len" => obj.len().to_value(),
                "listing" => obj.listing().to_value(),
                "selection-mode" => self.selection_mode.get().to_value(),
                "num-selected" => obj.num_selected().to_value(),
                _ => unimplemented!(),
            }
        }
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            model::SelectableList::bootstrap(obj);
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
        @implements gio::ListModel, model::SelectableList;
}

impl From<Option<&model::Client>> for ImageList {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::new(&[("client", &client)]).expect("Failed to create ImageList")
    }
}

impl ImageList {
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
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _, image)) = list.shift_remove_full(id) {
            image.emit_deleted();
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
                        .images()
                        .list(&podman::opts::ImageListOpts::builder().all(true).build())
                        .await
                }
            },
            clone!(@weak self as obj => move |result| {
                obj.set_listing(false);
                match result {
                    Ok(summaries) => {
                        let index = obj.len();
                        let mut added = 0;

                        let mut list = obj.imp().list.borrow_mut();
                        summaries.into_iter().for_each(|summary| {
                            if let Entry::Vacant(e) =
                                list.entry(summary.id.as_ref().unwrap().to_owned())
                            {
                                let image = model::Image::new(&obj, summary);

                                e.insert(image.clone());
                                obj.image_added(&image);

                                added += 1;
                            }
                        });

                        drop(list);
                        obj.items_changed(index, 0, added as u32);
                    }
                    Err(e) => {
                        log::error!("Error on retrieving images: {}", e);
                        err_op(super::RefreshError);
                    }
                }
            }),
        );
    }

    pub(crate) fn handle_event<F>(&self, event: podman::models::Event, err_op: F)
    where
        F: FnOnce(super::RefreshError) + Clone + 'static,
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
