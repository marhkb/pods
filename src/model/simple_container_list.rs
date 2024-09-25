use std::borrow::Borrow;
use std::cell::RefCell;
use std::sync::OnceLock;

use gio::prelude::*;
use gio::subclass::prelude::*;
use gtk::gio;
use gtk::glib;
use indexmap::map::IndexMap;

use crate::model;
use crate::model::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct SimpleContainerList(
        pub(super) RefCell<IndexMap<String, glib::WeakRef<model::Container>>>,
    );

    #[glib::object_subclass]
    impl ObjectSubclass for SimpleContainerList {
        const NAME: &'static str = "SimpleContainerList";
        type Type = super::SimpleContainerList;
        type Interfaces = (gio::ListModel, model::AbstractContainerList);
    }

    impl ObjectImpl for SimpleContainerList {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecUInt::builder("len").read_only().build(),
                    glib::ParamSpecUInt::builder("containers")
                        .read_only()
                        .build(),
                    glib::ParamSpecUInt::builder("created").read_only().build(),
                    glib::ParamSpecUInt::builder("dead").read_only().build(),
                    glib::ParamSpecUInt::builder("exited").read_only().build(),
                    glib::ParamSpecUInt::builder("not-running")
                        .read_only()
                        .build(),
                    glib::ParamSpecUInt::builder("paused").read_only().build(),
                    glib::ParamSpecUInt::builder("removing").read_only().build(),
                    glib::ParamSpecUInt::builder("running").read_only().build(),
                    glib::ParamSpecUInt::builder("stopped").read_only().build(),
                    glib::ParamSpecUInt::builder("stopping").read_only().build(),
                ]
            })
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "len" => obj.len().to_value(),
                "containers" => obj.containers().to_value(),
                "created" => obj.created().to_value(),
                "dead" => obj.dead().to_value(),
                "exited" => obj.exited().to_value(),
                "not-running" => obj.not_running().to_value(),
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
            model::AbstractContainerList::bootstrap(self.obj().upcast_ref());
        }
    }

    impl ListModelImpl for SimpleContainerList {
        fn item_type(&self) -> glib::Type {
            model::Container::static_type()
        }

        fn n_items(&self) -> u32 {
            self.0.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.0
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

impl Default for SimpleContainerList {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl SimpleContainerList {
    pub(crate) fn first_non_infra(&self) -> Option<model::Container> {
        self.imp()
            .0
            .borrow()
            .values()
            .filter_map(glib::WeakRef::upgrade)
            .find_map(|container| {
                if container.is_infra() {
                    None
                } else {
                    Some(container)
                }
            })
    }

    pub(crate) fn get(&self, index: usize) -> Option<model::Container> {
        self.imp()
            .0
            .borrow()
            .get_index(index)
            .map(|(_, c)| c)
            .and_then(glib::WeakRef::upgrade)
    }

    pub(crate) fn add_container(&self, container: &model::Container) {
        let (index, _) = self.imp().0.borrow_mut().insert_full(container.id(), {
            let weak_ref = glib::WeakRef::new();
            weak_ref.set(Some(container));
            weak_ref
        });

        self.items_changed(index as u32, 0, 1);
        self.container_added(container);
    }

    pub(crate) fn remove_container<Q: Borrow<str> + ?Sized>(&self, id: &Q) {
        let mut list = self.imp().0.borrow_mut();
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

    pub(crate) fn containers(&self) -> u32 {
        self.len()
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
            .0
            .borrow()
            .values()
            .filter_map(glib::WeakRef::upgrade)
            .filter(|container| container.status() == status)
            .count() as u32
    }
}
