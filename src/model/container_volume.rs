use std::cell::OnceCell;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;
use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ContainerVolume)]
    pub(crate) struct ContainerVolume {
        #[property(get, set, construct_only)]
        pub(super) container_volume_list: glib::WeakRef<model::ContainerVolumeList>,
        #[property(get, set, construct_only)]
        pub(super) volume: glib::WeakRef<model::Volume>,

        #[property(get, set, construct_only)]
        pub(super) destination: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) mode: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) rw: OnceCell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerVolume {
        const NAME: &'static str = "ContainerVolume";
        type Type = super::ContainerVolume;
    }

    impl ObjectImpl for ContainerVolume {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerVolume(ObjectSubclass<imp::ContainerVolume>);
}

impl ContainerVolume {
    pub(crate) fn new(
        container_volume_list: &model::ContainerVolumeList,
        volume: &model::Volume,
        dto: &engine::dto::Mount,
    ) -> Self {
        glib::Object::builder()
            .property("container-volume-list", container_volume_list)
            .property("volume", volume)
            .property("destination", &dto.destination)
            .property("mode", &dto.mode)
            .property("rw", dto.rw)
            .build()
    }
}
