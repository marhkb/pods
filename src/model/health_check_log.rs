use std::cell::OnceCell;

use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::Properties;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::podman;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::HealthCheckLog)]
    pub(crate) struct HealthCheckLog {
        #[property(get, set, construct_only)]
        pub(super) end: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) exit_code: OnceCell<i64>,
        #[property(get, set, construct_only)]
        pub(super) output: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) start: OnceCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HealthCheckLog {
        const NAME: &'static str = "HealthCheckLog";
        type Type = super::HealthCheckLog;
    }

    impl ObjectImpl for HealthCheckLog {
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
    pub(crate) struct HealthCheckLog(ObjectSubclass<imp::HealthCheckLog>)
        @implements gio::ListModel, model::SelectableList;
}

impl From<&podman::models::HealthCheckLog> for HealthCheckLog {
    fn from(data: &podman::models::HealthCheckLog) -> Self {
        glib::Object::builder()
            .property("end", data.end.as_ref().unwrap())
            .property("exit-code", data.exit_code.unwrap())
            .property("output", data.output.as_ref().unwrap())
            .property("start", data.start.as_ref().unwrap())
            .build()
    }
}
