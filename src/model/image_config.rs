use gtk::glib;
use gtk::prelude::ParamSpecBuilderExt;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::podman;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct ImageConfig {
        pub(super) cmd: OnceCell<Option<String>>,
        pub(super) entrypoint: OnceCell<Option<String>>,
        pub(super) exposed_ports: OnceCell<gtk::StringList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageConfig {
        const NAME: &'static str = "ImageConfig";
        type Type = super::ImageConfig;
    }

    impl ObjectImpl for ImageConfig {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::builder("cmd")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecString::builder("entrypoint")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecObject::builder::<gtk::StringList>("exposed-ports")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "cmd" => self.cmd.set(value.get().unwrap()).unwrap(),
                "entrypoint" => self.entrypoint.set(value.get().unwrap()).unwrap(),
                "exposed-ports" => self.exposed_ports.set(value.get().unwrap()).unwrap(),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
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
    pub(crate) fn from_libpod(config: podman::models::ImageConfig) -> Self {
        glib::Object::builder::<Self>()
            .property(
                "cmd",
                &utils::format_iter_or_none(
                    &mut config.cmd.as_deref().unwrap_or_default().iter(),
                    " ",
                ),
            )
            .property(
                "entrypoint",
                &utils::format_iter_or_none(
                    &mut config.entrypoint.as_deref().unwrap_or_default().iter(),
                    " ",
                ),
            )
            .property(
                "exposed-ports",
                &gtk::StringList::new(
                    &config
                        .exposed_ports
                        .unwrap_or_default()
                        .keys()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                ),
            )
            .build()
    }

    pub(crate) fn cmd(&self) -> Option<&str> {
        self.imp().cmd.get().unwrap().as_deref()
    }

    pub(crate) fn entrypoint(&self) -> Option<&str> {
        self.imp().entrypoint.get().unwrap().as_deref()
    }

    pub(crate) fn exposed_ports(&self) -> &gtk::StringList {
        self.imp().exposed_ports.get().unwrap()
    }
}
