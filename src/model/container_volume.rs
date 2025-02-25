use std::cell::OnceCell;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::model;
use crate::monad_boxed_type;
use crate::podman;

monad_boxed_type!(pub(crate) BoxedInspectMount(podman::models::InspectMount) impls Debug);

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
        pub(super) inner: OnceCell<BoxedInspectMount>,
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
        inner: podman::models::InspectMount,
    ) -> Self {
        glib::Object::builder()
            .property("container-volume-list", container_volume_list)
            .property("volume", volume)
            .property("inner", BoxedInspectMount::from(inner))
            .build()
    }
}
