use std::borrow::Borrow;
use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::sync::OnceLock;

use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::clone;
use glib::once_cell::sync::Lazy as SyncLazy;
use glib::subclass::Signal;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use indexmap::map::Entry;
use indexmap::IndexMap;

use crate::model;
use crate::model::prelude::*;
use crate::podman;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::VolumeList)]
    pub(crate) struct VolumeList {
        pub(super) list: RefCell<IndexMap<String, model::Volume>>,
        #[property(get, set)]
        pub(super) test: Cell<u32>,
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
    impl ObjectSubclass for VolumeList {
        const NAME: &'static str = "VolumeList";
        type Type = super::VolumeList;
        type Interfaces = (gio::ListModel, model::SelectableList);
    }

    impl ObjectImpl for VolumeList {
        fn signals() -> &'static [Signal] {
            static SIGNALS: SyncLazy<Vec<Signal>> = SyncLazy::new(|| {
                vec![
                    Signal::builder("volume-added")
                        .param_types([model::Volume::static_type()])
                        .build(),
                    Signal::builder("volume-removed")
                        .param_types([model::Volume::static_type()])
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                Self::derived_properties()
                    .iter()
                    .cloned()
                    .chain(vec![
                        glib::ParamSpecUInt::builder("len").read_only().build(),
                        glib::ParamSpecUInt::builder("unused").read_only().build(),
                        glib::ParamSpecUInt::builder("used").read_only().build(),
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
            match pspec.name() {
                "len" => self.obj().len().to_value(),
                "unused" => self.obj().unused().to_value(),
                "used" => self.obj().used().to_value(),
                "num-selected" => self.obj().num_selected().to_value(),
                _ => self.derived_property(id, pspec),
            }
        }
        fn constructed(&self) {
            self.parent_constructed();
            let obj = &*self.obj();

            model::SelectableList::bootstrap(obj.upcast_ref());

            obj.connect_items_changed(|self_, _, _, _| self_.notify("len"));

            obj.connect_volume_added(|list, _| list.notify_num_volumes());
            obj.connect_volume_removed(|list, _| list.notify_num_volumes());
        }
    }

    impl ListModelImpl for VolumeList {
        fn item_type(&self) -> glib::Type {
            model::Volume::static_type()
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

    impl VolumeList {
        pub(super) fn is_initialized(&self) -> bool {
            self.initialized.get().is_some()
        }

        pub(super) fn set_as_initialized(&self) {
            if self.is_initialized() {
                return;
            }
            self.initialized.set(()).unwrap();
            self.obj().notify("initialized");
        }

        pub(super) fn set_listing(&self, value: bool) {
            let obj = &*self.obj();
            if obj.listing() == value {
                return;
            }
            self.listing.set(value);
            obj.notify("listing");
        }
    }
}

glib::wrapper! {
    pub(crate) struct VolumeList(ObjectSubclass<imp::VolumeList>)
        @implements gio::ListModel, model::SelectableList;
}

impl From<&model::Client> for VolumeList {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl VolumeList {
    pub(crate) fn notify_num_volumes(&self) {
        self.notify("unused");
        self.notify("used");
    }

    pub(crate) fn len(&self) -> u32 {
        self.n_items()
    }

    pub(crate) fn unused(&self) -> u32 {
        self.len() - self.used()
    }

    pub(crate) fn used(&self) -> u32 {
        self.imp()
            .list
            .borrow()
            .values()
            .filter(|volume| volume.container_list().n_items() > 0)
            .count() as u32
    }

    pub(crate) fn get_volume<Q: Borrow<str> + ?Sized>(&self, name: &Q) -> Option<model::Volume> {
        self.imp().list.borrow().get(name.borrow()).cloned()
    }

    pub(crate) fn remove_volume(&self, id: &str) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _, volume)) = list.shift_remove_full(id) {
            drop(list);

            self.items_changed(idx as u32, 1, 0);
            self.volume_removed(&volume);
            volume.emit_deleted();
        }
    }

    pub(crate) fn refresh<F>(&self, err_op: F)
    where
        F: FnOnce(super::RefreshError) + Clone + 'static,
    {
        self.imp().set_listing(true);
        utils::do_async(
            {
                let podman = self.client().unwrap().podman();
                async move {
                    podman
                        .volumes()
                        .list(&podman::opts::VolumeListOpts::builder().build())
                        .await
                }
            },
            clone!(@weak self as obj => move |result| {
                match result {
                    Ok(volumes) => {
                        let imp = obj.imp();

                        let to_remove = imp
                            .list
                            .borrow()
                            .keys()
                            .filter(|name| {
                                !volumes
                                    .iter()
                                    .any(|volume| &volume.name == *name)
                            })
                            .cloned()
                            .collect::<Vec<_>>();
                        to_remove.iter().for_each(|name| {
                            obj.remove_volume(name);
                        });

                        volumes.into_iter().for_each(|volume| {
                            let index = obj.len();

                            let mut list = imp.list.borrow_mut();
                            if let Entry::Vacant(e) = list.entry(volume.name.clone()) {
                                let volume = model::Volume::new(&obj, volume);
                                e.insert(volume.clone());

                                drop(list);

                                obj.items_changed(index, 0, 1);
                                obj.volume_added(&volume);
                            }
                        });
                    }
                    Err(e) => {
                        log::error!("Error on retrieving volumes: {}", e);
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
        match event.action.as_str() {
            "remove" => self.remove_volume(event.actor.attributes.get("name").unwrap()),
            "create" => self.refresh(err_op),
            other => log::warn!("unhandled volume action: {other}"),
        }
    }

    fn volume_added(&self, volume: &model::Volume) {
        self.emit_by_name::<()>("volume-added", &[volume]);
    }

    pub(crate) fn connect_volume_added<F: Fn(&Self, &model::Volume) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("volume-added", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let volume = values[1].get::<model::Volume>().unwrap();
            f(&obj, &volume);

            None
        })
    }

    fn volume_removed(&self, volume: &model::Volume) {
        self.emit_by_name::<()>("volume-removed", &[volume]);
    }

    pub(crate) fn connect_volume_removed<F: Fn(&Self, &model::Volume) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("volume-removed", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let volume = values[1].get::<model::Volume>().unwrap();
            f(&obj, &volume);

            None
        })
    }
}
