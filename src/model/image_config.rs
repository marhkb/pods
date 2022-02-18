use gtk::glib;
use gtk::subclass::prelude::*;

use crate::utils;

mod imp {
    use gtk::prelude::{StaticType, ToValue};
    use once_cell::sync::Lazy;
    use once_cell::unsync::OnceCell;

    use super::*;

    #[derive(Debug, Default)]
    pub struct ImageConfig {
        pub cmd: OnceCell<Option<String>>,
        pub entrypoint: OnceCell<Option<String>>,
        pub exposed_ports: OnceCell<utils::BoxedStringBTreeSet>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageConfig {
        const NAME: &'static str = "ImageConfig";
        type Type = super::ImageConfig;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for ImageConfig {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::new(
                        "cmd",
                        "Cmd",
                        "The command of this ImageConfig",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "entrypoint",
                        "Entrypoint",
                        "The entrypoint of this ImageConfig",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecBoxed::new(
                        "exposed-ports",
                        "Exposed Ports",
                        "The exposed _ports of this ImageConfig",
                        utils::BoxedStringBTreeSet::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "cmd" => self.cmd.set(value.get().unwrap()).unwrap(),
                "entrypoint" => self.entrypoint.set(value.get().unwrap()).unwrap(),
                "exposed-ports" => self.exposed_ports.set(value.get().unwrap()).unwrap(),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "cmd" => obj.cmd().to_value(),
                "entrypoint" => obj.entrypoint().to_value(),
                "exposed-ports" => obj.exposed_ports().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct ImageConfig(ObjectSubclass<imp::ImageConfig>);
}

impl ImageConfig {
    pub fn from_libpod(config: podman_api::models::ImageConfig) -> Self {
        glib::Object::new(&[
            (
                "cmd",
                &utils::format_iter_or_none(
                    &mut config.cmd.as_deref().unwrap_or_default().iter(),
                    " ",
                ),
            ),
            (
                "entrypoint",
                &utils::format_iter_or_none(
                    &mut config.entrypoint.as_deref().unwrap_or_default().iter(),
                    " ",
                ),
            ),
            (
                "exposed-ports",
                &utils::BoxedStringBTreeSet(
                    config
                        .exposed_ports
                        .map(|ports| ports.into_keys().collect())
                        .unwrap_or_default(),
                ),
            ),
        ])
        .expect("Failed to create ImageConfig")
    }

    pub fn cmd(&self) -> Option<&str> {
        self.imp().cmd.get().unwrap().as_deref()
    }

    pub fn entrypoint(&self) -> Option<&str> {
        self.imp().entrypoint.get().unwrap().as_deref()
    }

    pub fn exposed_ports(&self) -> &utils::BoxedStringBTreeSet {
        self.imp().exposed_ports.get().unwrap()
    }
}
