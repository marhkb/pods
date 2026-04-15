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
    #[properties(wrapper_type = super::Info)]
    pub(crate) struct Info {
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,

        #[property(get, set, construct_only, nullable)]
        pub(super) arch: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) cgroup_driver: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) cgroup_version: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) hostname: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) kernel: OnceCell<Option<String>>,
        #[property(get, set, construct_only)]
        pub(super) memory: OnceCell<i64>,
        #[property(get, set, construct_only)]
        pub(super) cpus: OnceCell<i64>,
        #[property(get, set, construct_only, nullable)]
        pub(super) os: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) storage_driver: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) storage_root_dir: OnceCell<Option<String>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) version: OnceCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Info {
        const NAME: &'static str = "Info";
        type Type = super::Info;
    }

    impl ObjectImpl for Info {
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
    pub(crate) struct Info(ObjectSubclass<imp::Info>);
}

impl Info {
    pub(crate) fn new(client: &model::Client, value: engine::dto::Info) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("arch", value.arch)
            .property("cgroup-driver", value.cgroup_driver)
            .property("cgroup-version", value.cgroup_version)
            .property("hostname", value.hostname)
            .property("kernel", value.kernel)
            .property(
                "memory",
                value
                    .mem_total
                    .map(|mem_total| mem_total as i64)
                    .unwrap_or(-1),
            )
            .property("cpus", value.cpus.map(|cpus| cpus as i64).unwrap_or(-1))
            .property("os", value.os)
            .property("storage-driver", value.storage_driver)
            .property("storage-root-dir", value.storage_root_dir)
            .property("version", value.version)
            .build()
    }
}

impl Info {
    pub(crate) fn api(&self) -> Option<engine::Engine> {
        self.client().map(|client| (*client.engine()).clone())
    }
}
