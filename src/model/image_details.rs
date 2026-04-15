use std::cell::Cell;
use std::cell::OnceCell;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ImageDetails)]
    pub(crate) struct ImageDetails {
        #[property(get, set, construct_only, nullable)]
        pub(super) architecture: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) author: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) cmd: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) comment: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) entrypoint: OnceCell<Option<String>>,
        #[property(get, set, construct_only)]
        pub(super) exposed_ports: OnceCell<gtk::StringList>,
        #[property(get, set, construct)]
        pub(super) shared_size: Cell<u64>,
        #[property(get, set, construct)]
        pub(super) virtual_size: Cell<u64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageDetails {
        const NAME: &'static str = "ImageDetails";
        type Type = super::ImageDetails;
    }

    impl ObjectImpl for ImageDetails {
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
    pub(crate) struct ImageDetails(ObjectSubclass<imp::ImageDetails>);
}

impl From<engine::dto::ImageDetails> for ImageDetails {
    fn from(value: engine::dto::ImageDetails) -> Self {
        glib::Object::builder()
            .property("architecture", value.architecture)
            .property("author", value.author)
            .property("cmd", value.cmd)
            .property("comment", value.comment)
            .property("entrypoint", value.entrypoint)
            .property(
                "exposed-ports",
                gtk::StringList::from_iter(value.exposed_ports),
            )
            .property("shared-size", value.shared_size.unwrap_or(0))
            .property("virtual-size", value.virtual_size.unwrap_or(0))
            .build()
    }
}

impl ImageDetails {
    pub(crate) fn update(&self, dto: engine::dto::ImageDetails) {
        self.set_shared_size(dto.shared_size.unwrap_or(0));
        self.set_virtual_size(dto.virtual_size.unwrap_or(0));
    }
}
