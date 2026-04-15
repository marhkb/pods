use std::cell::OnceCell;
use std::ops::Deref;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;
use crate::model;
use crate::monad_boxed_type;

monad_boxed_type!(pub(crate) BoxedEngine(engine::Engine) impls Debug);

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Engine)]
    pub(crate) struct Engine {
        #[property(get, set, construct_only)]
        pub(super) inner: OnceCell<BoxedEngine>,
        #[property(get, set, construct_only, default)]
        pub(super) typ: OnceCell<model::EngineType>,
        #[property(get, set, construct_only)]
        pub(super) capabilities: OnceCell<model::EngineCapabilities>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Engine {
        const NAME: &'static str = "Engine";
        type Type = super::Engine;
    }

    impl ObjectImpl for Engine {
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
    pub(crate) struct Engine(ObjectSubclass<imp::Engine>);
}

impl From<engine::Engine> for Engine {
    fn from(value: engine::Engine) -> Self {
        glib::Object::builder()
            .property(
                "capabilities",
                model::EngineCapabilities::from(value.capabilities()),
            )
            .property("typ", model::EngineType::from(&value))
            .property("inner", BoxedEngine::from(value))
            .build()
    }
}

impl Deref for Engine {
    type Target = engine::Engine;

    fn deref(&self) -> &Self::Target {
        let imp = self.imp();
        imp.inner.get().unwrap()
    }
}
