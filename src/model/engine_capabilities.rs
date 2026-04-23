use std::cell::OnceCell;
use std::marker::PhantomData;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;
use crate::monad_boxed_type;

monad_boxed_type!(pub(crate) BoxedCapabilities(engine::Capabilities) impls Debug);

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::EngineCapabilities)]
    pub(crate) struct EngineCapabilities {
        #[property(get, set, construct_only)]
        pub(super) inner: OnceCell<BoxedCapabilities>,

        #[property(get = Self::kube_generation)]
        _kube_generation: PhantomData<bool>,
        #[property(get = Self::manual_health_check)]
        _manual_health_check: PhantomData<bool>,
        #[property(get = Self::pods)]
        _pods: PhantomData<bool>,
        #[property(get = Self::privileged_containers)]
        _privileged_containers: PhantomData<bool>,
        #[property(get = Self::prune_external_images)]
        _prune_external_images: PhantomData<bool>,
        #[property(get = Self::push_image_with_tls_verify)]
        _push_image_tls_verify: PhantomData<bool>,
        #[property(get = Self::prune_all_volumes)]
        _prune_all_volumes: PhantomData<bool>,
        #[property(get = Self::prune_volumes_until)]
        _prune_volumes_until: PhantomData<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EngineCapabilities {
        const NAME: &'static str = "EngineCapabilities";
        type Type = super::EngineCapabilities;
    }

    impl ObjectImpl for EngineCapabilities {
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

    impl EngineCapabilities {
        pub(super) fn kube_generation(&self) -> bool {
            self.obj().inner().kube_generation
        }

        pub(super) fn manual_health_check(&self) -> bool {
            self.obj().inner().manual_health_check
        }

        pub(super) fn pods(&self) -> bool {
            self.obj().inner().pods
        }

        pub(super) fn privileged_containers(&self) -> bool {
            self.obj().inner().privileged_containers
        }

        pub(super) fn prune_external_images(&self) -> bool {
            self.obj().inner().prune_external_images
        }

        pub(super) fn push_image_with_tls_verify(&self) -> bool {
            self.obj().inner().push_image_with_tls_verify
        }

        pub(super) fn prune_all_volumes(&self) -> bool {
            self.obj().inner().prune_all_volumes
        }

        pub(super) fn prune_volumes_until(&self) -> bool {
            self.obj().inner().prune_volumes_until
        }
    }
}

glib::wrapper! {
    pub(crate) struct EngineCapabilities(ObjectSubclass<imp::EngineCapabilities>);
}

impl From<engine::Capabilities> for EngineCapabilities {
    fn from(value: engine::Capabilities) -> Self {
        glib::Object::builder()
            .property("inner", BoxedCapabilities::from(value))
            .build()
    }
}
