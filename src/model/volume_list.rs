use std::borrow::Borrow;
use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::sync::OnceLock;

use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use glib::subclass::Signal;
use gtk::gio;
use gtk::glib;
use indexmap::IndexMap;

use crate::engine;
use crate::model;
use crate::model::prelude::*;
use crate::rt;

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
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("volume-added")
                        .param_types([model::Volume::static_type()])
                        .build(),
                    Signal::builder("volume-removed")
                        .param_types([model::Volume::static_type()])
                        .build(),
                    Signal::builder("containers-of-volume-changed")
                        .param_types([model::Volume::static_type()])
                        .build(),
                ]
            })
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

    fn add_volume(&self, inspection: engine::dto::Volume) {
        let volume = model::Volume::new(self, inspection);

        let index = self.len();

        self.imp()
            .list
            .borrow_mut()
            .insert(volume.name(), volume.clone());

        self.items_changed(index, 0, 1);
        self.volume_added(&volume);
    }

    fn inspect_and_add_volume<F>(&self, name: String, err_op: F)
    where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        let Some(api) = self.api() else { return };

        rt::Promise::new(async move { api.get(&name).inspect().await }).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |inspection| match inspection {
                Ok(inspection) => obj.add_volume(inspection),
                Err(e) => err_op(e),
            }
        ));
    }

    pub(crate) fn get_volume<Q: Borrow<str> + ?Sized>(&self, name: &Q) -> Option<model::Volume> {
        self.imp().list.borrow().get(name.borrow()).cloned()
    }

    pub(crate) fn remove_volume(&self, id: &str) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _, volume)) = list.shift_remove_full(id) {
            drop(list);

            self.items_changed(idx as u32, 1, 0);
            self.emit_by_name::<()>("volume-removed", &[&volume]);
            volume.emit_deleted();
        }
    }

    pub(crate) fn refresh<F>(&self, err_op: F)
    where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        let Some(api) = self.api() else { return };

        self.imp().set_listing(true);

        rt::Promise::new(async move { api.list().await }).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |result| {
                match result {
                    Ok(volumes) => {
                        let to_remove = obj
                            .imp()
                            .list
                            .borrow()
                            .keys()
                            .filter(|name| !volumes.iter().any(|volume| &volume.name == *name))
                            .cloned()
                            .collect::<Vec<_>>();
                        to_remove.iter().for_each(|name| {
                            obj.remove_volume(name);
                        });

                        volumes.into_iter().for_each(|volume| {
                            if obj.get_volume(&volume.name).is_none() {
                                obj.add_volume(volume);
                            }
                        });
                    }
                    Err(e) => {
                        log::error!("Error on retrieving volumes: {}", e);
                        err_op(e);
                    }
                }
                let imp = obj.imp();
                imp.set_listing(false);
                imp.set_as_initialized();
            }
        ));
    }

    pub(crate) fn handle_event<F>(&self, event: engine::dto::Event, err_op: F)
    where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        match event {
            engine::Response::Docker(event) => {
                let name = event.actor.unwrap().id.unwrap();

                match event.action.unwrap().as_str() {
                    "destroy" => self.remove_volume(&name),
                    "create" => self.inspect_and_add_volume(name, err_op),
                    other => log::warn!("unhandled volume action: {other}"),
                }
            }
            engine::Response::Podman(mut event) => {
                let name = event.actor.attributes.remove("name").unwrap();

                match event.action.as_str() {
                    "remove" => self.remove_volume(&name),
                    "create" => self.inspect_and_add_volume(name, err_op),
                    other => log::warn!("unhandled volume action: {other}"),
                }
            }
        }
    }

    fn volume_added(&self, volume: &model::Volume) {
        self.emit_by_name::<()>("volume-added", &[volume]);
        volume.container_list().connect_notify_local(
            Some("len"),
            clone!(
                #[weak(rename_to=obj)]
                self,
                #[weak]
                volume,
                move |_, _| {
                    obj.emit_by_name::<()>("containers-of-volume-changed", &[&volume]);
                }
            ),
        );
    }

    pub(crate) fn connect_volume_added<F: Fn(&Self, &model::Volume) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_signal("volume-added", f)
    }

    pub(crate) fn connect_volume_removed<F: Fn(&Self, &model::Volume) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_signal("volume-removed", f)
    }

    pub(crate) fn connect_containers_of_volume_changed<F: Fn(&Self, &model::Volume) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_signal("containers-of-volume-changed", f)
    }

    fn connect_signal<F: Fn(&Self, &model::Volume) + 'static>(
        &self,
        signal: &str,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local(signal, true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let volume = values[1].get::<model::Volume>().unwrap();
            f(&obj, &volume);

            None
        })
    }

    pub(crate) fn api(&self) -> Option<engine::api::Volumes> {
        self.client()
            .as_ref()
            .map(model::Client::engine)
            .as_deref()
            .map(engine::Engine::volumes)
    }
}
