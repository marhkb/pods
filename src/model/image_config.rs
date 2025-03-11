use std::cell::OnceCell;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::podman;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ImageConfig)]
    pub(crate) struct ImageConfig {
        #[property(get, set, construct_only, nullable)]
        pub(super) cmd: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) entrypoint: OnceCell<Option<String>>,
        #[property(get, set, construct_only)]
        pub(super) exposed_ports: OnceCell<gtk::StringList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageConfig {
        const NAME: &'static str = "ImageConfig";
        type Type = super::ImageConfig;
    }

    impl ObjectImpl for ImageConfig {
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
    pub(crate) struct ImageConfig(ObjectSubclass<imp::ImageConfig>);
}

impl ImageConfig {
    pub(crate) fn from_libpod(config: &podman::models::ImageConfig) -> Self {
        glib::Object::builder()
            .property(
                "cmd",
                utils::format_iter_or_none(config.cmd.as_deref().unwrap_or_default().iter(), " "),
            )
            .property(
                "entrypoint",
                utils::format_iter_or_none(
                    config.entrypoint.as_deref().unwrap_or_default().iter(),
                    " ",
                ),
            )
            .property(
                "exposed-ports",
                gtk::StringList::new(
                    &config
                        .exposed_ports
                        .as_ref()
                        .map(|ports| ports.keys().map(String::as_str).collect::<Vec<_>>())
                        .unwrap_or_default(),
                ),
            )
            .build()
    }
}
