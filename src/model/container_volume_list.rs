use std::cell::RefCell;

use gio::prelude::*;
use gio::subclass::prelude::*;
use gtk::gio;
use gtk::glib;
use indexmap::map::IndexMap;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct ContainerVolumeList(
        pub(super) RefCell<IndexMap<String, model::ContainerVolume>>,
    );

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerVolumeList {
        const NAME: &'static str = "ContainerVolumeList";
        type Type = super::ContainerVolumeList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for ContainerVolumeList {}

    impl ListModelImpl for ContainerVolumeList {
        fn item_type(&self) -> glib::Type {
            model::ContainerVolume::static_type()
        }

        fn n_items(&self) -> u32 {
            self.0.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.0
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
        glib::Object::builder().build()
    }
}

impl ContainerVolumeList {
    pub(crate) fn add_volume(&self, container_volume: model::ContainerVolume) {
        if let Some(ref volume) = container_volume.volume() {
            let (index, _) = self
                .imp()
                .0
                .borrow_mut()
                .insert_full(volume.inner().name.clone(), container_volume);

            self.items_changed(index as u32, 0, 1);
        }
    }
}
