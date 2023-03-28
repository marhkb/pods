use glib::Properties;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::unsync::OnceCell as UnsyncOnceCell;

use crate::model;
use crate::podman;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::HealthCheckLog)]
    pub(crate) struct HealthCheckLog {
        #[property(get, set, construct_only)]
        pub(super) end: UnsyncOnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) exit_code: UnsyncOnceCell<i64>,
        #[property(get, set, construct_only)]
        pub(super) output: UnsyncOnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) start: UnsyncOnceCell<String>,
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
