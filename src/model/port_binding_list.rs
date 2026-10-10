use std::cell::RefCell;
use std::marker::PhantomData;

use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::Properties;
use gtk::gio;
use gtk::glib;

use crate::engine;
use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::PortBindingList)]
    pub(crate) struct PortBindingList {
        #[property(get, set, construct_only, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,

        #[property(get = Self::len)]
        _len: PhantomData<u32>,

        pub(super) list: RefCell<Vec<model::PortBinding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PortBindingList {
        const NAME: &'static str = "ContainerPortBindingList";
        type Type = super::PortBindingList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for PortBindingList {
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
            self.obj()
                .connect_items_changed(|obj, _, _, _| obj.notify_len());
        }
    }

    impl ListModelImpl for PortBindingList {
        fn item_type(&self) -> glib::Type {
            model::PortBinding::static_type()
        }

        fn n_items(&self) -> u32 {
            self.obj().len()
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.obj().get(position as usize).map(|obj| obj.upcast())
        }
    }

    impl PortBindingList {
        pub(super) fn len(&self) -> u32 {
            self.list.borrow().len() as u32
        }
    }
}

glib::wrapper! {
    pub(crate) struct PortBindingList(ObjectSubclass<imp::PortBindingList>)
        @implements gio::ListModel;
}

impl From<&model::Container> for PortBindingList {
    fn from(value: &model::Container) -> Self {
        glib::Object::builder().property("container", value).build()
    }
}

impl PortBindingList {
    pub(crate) fn update(&self, port_mappings: Vec<engine::dto::PortBinding>) {
        let removed = self.len();
        let added = port_mappings.len() as u32;

        if added <= removed {
            return;
        }

        let mut list = self.imp().list.borrow_mut();

        list.clear();
        list.append(
            &mut port_mappings
                .into_iter()
                .map(|port_binding| model::PortBinding::new(self, port_binding))
                .collect(),
        );

        drop(list);

        self.items_changed(0, removed, added);
    }

    pub(crate) fn get(&self, index: usize) -> Option<model::PortBinding> {
        self.imp().list.borrow().get(index).cloned()
    }
}
