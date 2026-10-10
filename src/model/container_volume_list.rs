use std::cell::RefCell;
use std::marker::PhantomData;

use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use indexmap::map::IndexMap;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ContainerVolumeList)]
    pub(crate) struct ContainerVolumeList {
        pub(super) list: RefCell<IndexMap<String, model::ContainerVolume>>,

        #[property(get = Self::n_items)]
        _len: PhantomData<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerVolumeList {
        const NAME: &'static str = "ContainerVolumeList";
        type Type = super::ContainerVolumeList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for ContainerVolumeList {
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

    impl ListModelImpl for ContainerVolumeList {
        fn item_type(&self) -> glib::Type {
            model::ContainerVolume::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .map(|(_, obj)| obj.clone().upcast())
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerVolumeList(ObjectSubclass<imp::ContainerVolumeList>)
        @implements gio::ListModel;
}

impl Default for ContainerVolumeList {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ContainerVolumeList {
    pub(crate) fn add_volume(&self, container_volume: model::ContainerVolume) {
        let Some(ref volume) = container_volume.volume() else {
            return;
        };

        let (index, _) = self
            .imp()
            .list
            .borrow_mut()
            .insert_full(volume.name(), container_volume);

        self.items_changed(index as u32, 0, 1);
    }
}
