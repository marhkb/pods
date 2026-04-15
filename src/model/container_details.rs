use std::cell::Cell;
use std::cell::OnceCell;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;
use crate::model;
use crate::monad_boxed_type;

monad_boxed_type!(pub(crate) BoxedHealthConfig(engine::dto::HealthConfig) impls Debug is nullable);

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ContainerDetails)]
    pub(crate) struct ContainerDetails {
        #[property(get, set, construct_only)]
        pub(super) health_check_logs: OnceCell<model::HealthCheckLogList>,
        #[property(get, set, construct_only)]
        pub(super) health_config: OnceCell<Option<BoxedHealthConfig>>,
        #[property(get, set)]
        pub(super) health_failing_streak: Cell<u32>,
        #[property(get, set, construct)]
        pub(super) size: Cell<i64>,
        #[property(get, set, construct)]
        pub(super) up_since: Cell<i64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerDetails {
        const NAME: &'static str = "ContainerDetails";
        type Type = super::ContainerDetails;
    }

    impl ObjectImpl for ContainerDetails {
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
    pub(crate) struct ContainerDetails(ObjectSubclass<imp::ContainerDetails>);
}

impl From<engine::dto::ContainerDetails> for ContainerDetails {
    fn from(value: engine::dto::ContainerDetails) -> Self {
        glib::Object::builder()
            .property(
                "health-check-logs",
                model::HealthCheckLogList::from(value.health_check_logs),
            )
            .property("health-config", value.health_config.map(BoxedHealthConfig))
            .property("health-failing-streak", value.health_failing_streak)
            .property("size", value.size)
            .property("up-since", value.up_since)
            .build()
    }
}

impl ContainerDetails {
    pub(crate) fn update(&self, dto: engine::dto::ContainerDetails) {
        self.health_check_logs().sync(dto.health_check_logs);
        self.set_health_failing_streak(dto.health_failing_streak);
        self.set_size(dto.size);
        self.set_up_since(dto.up_since);
    }
}
