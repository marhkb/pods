use std::borrow::Borrow;
use std::cell::RefCell;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use indexmap::map::IndexMap;
use once_cell::sync::Lazy;

use super::AbstractContainerListExt;
use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct SimpleContainerList {
        pub(super) client: glib::WeakRef<model::Client>,
        pub(super) list: RefCell<IndexMap<String, glib::WeakRef<model::Container>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SimpleContainerList {
        const NAME: &'static str = "SimpleContainerList";
        type Type = super::SimpleContainerList;
        type Interfaces = (gio::ListModel, model::AbstractContainerList);
    }

    impl ObjectImpl for SimpleContainerList {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Client>("client")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecUInt::builder("len")
                        .flags(glib::ParamFlags::READABLE)
                        .build(),
                    glib::ParamSpecUInt::builder("created")
                        .flags(glib::ParamFlags::READABLE)
                        .build(),
                    glib::ParamSpecUInt::builder("dead")
                        .flags(glib::ParamFlags::READABLE)
                        .build(),
                    glib::ParamSpecUInt::builder("exited")
                        .flags(glib::ParamFlags::READABLE)
                        .build(),
                    glib::ParamSpecUInt::builder("paused")
                        .flags(glib::ParamFlags::READABLE)
                        .build(),
                    glib::ParamSpecUInt::builder("removing")
                        .flags(glib::ParamFlags::READABLE)
                        .build(),
                    glib::ParamSpecUInt::builder("running")
                        .flags(glib::ParamFlags::READABLE)
                        .build(),
                    glib::ParamSpecUInt::builder("stopped")
                        .flags(glib::ParamFlags::READABLE)
                        .build(),
                    glib::ParamSpecUInt::builder("stopping")
                        .flags(glib::ParamFlags::READABLE)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "client" => self.client.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "client" => obj.client().to_value(),
                "len" => obj.len().to_value(),
                "created" => obj.created().to_value(),
                "dead" => obj.dead().to_value(),
                "exited" => obj.exited().to_value(),
                "paused" => obj.paused().to_value(),
                "removing" => obj.removing().to_value(),
                "running" => obj.running().to_value(),
                "stopped" => obj.stopped().to_value(),
                "stopping" => obj.stopping().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            model::AbstractContainerList::bootstrap(&*self.obj());
        }
    }

    impl ListModelImpl for SimpleContainerList {
        fn item_type(&self) -> glib::Type {
            model::Container::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .and_then(|(_, obj)| obj.upgrade().map(|c| c.upcast()))
        }
    }
}

glib::wrapper! {
    pub(crate) struct SimpleContainerList(ObjectSubclass<imp::SimpleContainerList>)
        @implements gio::ListModel, model::AbstractContainerList;
}

impl From<Option<&model::Client>> for SimpleContainerList {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::builder::<Self>()
            .property("client", &client)
            .build()
    }
}

impl SimpleContainerList {
    pub(crate) fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    pub(crate) fn get(&self, index: usize) -> Option<model::Container> {
        self.imp()
            .list
            .borrow()
            .get_index(index)
            .map(|(_, c)| c)
            .and_then(glib::WeakRef::upgrade)
    }

    pub(crate) fn add_container(&self, container: &model::Container) {
        let (index, _) = self
            .imp()
            .list
            .borrow_mut()
            .insert_full(container.id().to_owned(), {
                let weak_ref = glib::WeakRef::new();
                weak_ref.set(Some(container));
                weak_ref
            });

        self.items_changed(index as u32, 0, 1);
        self.container_added(container);
    }

    pub(crate) fn remove_container<Q: Borrow<str> + ?Sized>(&self, id: &Q) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _, container)) = list.shift_remove_full(id.borrow()) {
            drop(list);
            self.items_changed(idx as u32, 1, 0);
            if let Some(container) = container.upgrade() {
                self.container_removed(&container);
            }
        }
    }

    pub(crate) fn len(&self) -> u32 {
        self.n_items()
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

    pub(crate) fn removing(&self) -> u32 {
        self.num_containers_of_status(model::ContainerStatus::Removing)
    }

    pub(crate) fn running(&self) -> u32 {
        self.num_containers_of_status(model::ContainerStatus::Running)
    }

    pub(crate) fn stopped(&self) -> u32 {
        self.num_containers_of_status(model::ContainerStatus::Stopped)
    }

    pub(crate) fn stopping(&self) -> u32 {
        self.num_containers_of_status(model::ContainerStatus::Stopping)
    }

    pub(crate) fn num_containers_of_status(&self, status: model::ContainerStatus) -> u32 {
        self.imp()
            .list
            .borrow()
            .values()
            .filter_map(glib::WeakRef::upgrade)
            .filter(|container| container.status() == status)
            .count() as u32
    }
}
