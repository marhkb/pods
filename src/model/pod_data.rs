use gtk::glib::{self};
use gtk::prelude::ParamSpecBuilderExt;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::podman;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct PodData {
        pub(super) hostname: OnceCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodData {
        const NAME: &'static str = "PodData";
        type Type = super::PodData;
    }

    impl ObjectImpl for PodData {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecString::builder("hostname")
                    .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "hostname" => self.hostname.set(value.get().unwrap()).unwrap(),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "hostname" => self.instance().hostname().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct PodData(ObjectSubclass<imp::PodData>);
}

impl From<podman::models::InspectPodData> for PodData {
    fn from(data: podman::models::InspectPodData) -> Self {
        glib::Object::builder::<Self>()
            .property("hostname", &data.hostname.unwrap_or_default())
            .build()
    }
}

impl PodData {
    pub(crate) fn hostname(&self) -> &str {
        self.imp().hostname.get().unwrap()
    }
}
