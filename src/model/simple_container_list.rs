use std::borrow::Borrow;
use std::cell::RefCell;
use std::marker::PhantomData;

use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use indexmap::map::Entry;
use indexmap::map::IndexMap;

use crate::model;
use crate::model::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::SimpleContainerList)]
    pub(crate) struct SimpleContainerList {
        pub(super) list: RefCell<IndexMap<String, glib::WeakRef<model::Container>>>,

        #[property(get = Self::n_items)]
        _containers: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Created))]
        _created: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Dead))]
        _dead: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Exited))]
        _exited: PhantomData<u32>,
        #[property(get = Self::n_items)]
        _len: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.n_items() - imp.obj().running())]
        _not_running: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Paused))]
        _paused: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Removing))]
        _removing: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Running))]
        _running: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Stopped))]
        _stopped: PhantomData<u32>,
        #[property(get = |imp: &Self| imp.num_containers_of_status(model::ContainerStatus::Stopping))]
        _stopping: PhantomData<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SimpleContainerList {
        const NAME: &'static str = "PdsSimpleContainerList";
        type Type = super::SimpleContainerList;
        type Interfaces = (gio::ListModel, model::AbstractContainerList);
    }

    impl ObjectImpl for SimpleContainerList {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
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
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .and_then(|(_, obj)| obj.upgrade().map(|c| c.upcast()))
        }
    }

    impl SimpleContainerList {
        fn num_containers_of_status(&self, status: model::ContainerStatus) -> u32 {
            self.list
                .borrow()
                .values()
                .filter_map(glib::WeakRef::upgrade)
                .filter(|container| container.status() == status)
                .count() as u32
        }
    }
}

glib::wrapper! {
    pub(crate) struct SimpleContainerList(ObjectSubclass<imp::SimpleContainerList>)
        @implements gio::ListModel, model::AbstractContainerList;
}

impl Default for SimpleContainerList {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl SimpleContainerList {
    pub(crate) fn first_non_infra(&self) -> Option<model::Container> {
        self.imp()
            .list
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
            .list
            .borrow()
            .get_index(index)
            .map(|(_, c)| c)
            .and_then(glib::WeakRef::upgrade)
    }

    pub(crate) fn add_container(&self, container: &model::Container) {
        let position = {
            let mut list = self.imp().list.borrow_mut();
            let Entry::Vacant(e) = list.entry(container.id()) else {
                return;
            };

            let position = e.index() as u32;

            e.insert({
                let weak_ref = glib::WeakRef::new();
                weak_ref.set(Some(container));
                weak_ref
            });

            position
        };

        self.items_changed(position, 0, 1);
        self.container_added(container);
    }

    pub(crate) fn remove_container<Q: Borrow<str> + ?Sized>(&self, id: &Q) {
        let (position, container) = {
            let mut list = self.imp().list.borrow_mut();
            let Some((idx, _, container)) = list.shift_remove_full(id.borrow()) else {
                return;
            };

            (idx as u32, container)
        };

        self.items_changed(position, 1, 0);
        if let Some(container) = container.upgrade() {
            self.container_removed(&container);
        }
    }
}
