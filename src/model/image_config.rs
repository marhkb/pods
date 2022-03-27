use std::collections::BTreeSet;

use gtk::glib;
use gtk::prelude::{StaticType, ToValue};
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::{api, utils};

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct ImageConfig {
        pub(super) cmd: OnceCell<Option<String>>,
        pub(super) entrypoint: OnceCell<Option<String>>,
        pub(super) exposed_ports: OnceCell<utils::BoxedStringBTreeSet>,
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
    pub(crate) struct ImageConfig(ObjectSubclass<imp::ImageConfig>);
}

impl ImageConfig {
    pub(crate) fn from_libpod(config: api::ImageConfig) -> Self {
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
                &utils::BoxedStringBTreeSet::from(
                    config
                        .exposed_ports
                        .map(|ports| ports.into_keys().collect::<BTreeSet<_>>())
                        .unwrap_or_default(),
                ),
            ),
        ])
        .expect("Failed to create ImageConfig")
    }

    pub(crate) fn cmd(&self) -> Option<&str> {
        self.imp().cmd.get().unwrap().as_deref()
    }

    pub(crate) fn entrypoint(&self) -> Option<&str> {
        self.imp().entrypoint.get().unwrap().as_deref()
    }

    pub(crate) fn exposed_ports(&self) -> &utils::BoxedStringBTreeSet {
        self.imp().exposed_ports.get().unwrap()
    }
}
