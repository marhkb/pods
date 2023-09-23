use std::cell::Cell;
use std::cell::OnceCell;
use std::collections::HashMap;

use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::Properties;
use gtk::glib;

use crate::model;
use crate::monad_boxed_type;
use crate::podman;

monad_boxed_type!(pub(crate) BoxedSchema2HealthConfig(podman::models::Schema2HealthConfig) impls Debug is nullable);
monad_boxed_type!(pub(crate) BoxedPortBindings(HashMap<String, Option<Vec<podman::models::InspectHostPort>>>) impls Debug is nullable);
monad_boxed_type!(pub(crate) BoxedInspectMounts(HashMap<String, podman::models::InspectMount>) impls Debug);

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ContainerData)]
    pub(crate) struct ContainerData {
        pub(super) health_check_log_list: model::HealthCheckLogList,
        #[property(get, set, construct_only)]
        pub(super) health_config: OnceCell<Option<BoxedSchema2HealthConfig>>,
        #[property(get, set, construct_only)]
        pub(super) health_failing_streak: Cell<u32>,
        #[property(get, set, construct_only)]
        pub(super) mounts: OnceCell<BoxedInspectMounts>,
        #[property(get, set, construct_only)]
        pub(super) port_bindings: OnceCell<Option<BoxedPortBindings>>,
        #[property(get, set, construct_only)]
        pub(super) size: OnceCell<i64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerData {
        const NAME: &'static str = "ContainerData";
        type Type = super::ContainerData;
    }

    impl ObjectImpl for ContainerData {
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
    pub(crate) struct ContainerData(ObjectSubclass<imp::ContainerData>);
}

impl From<podman::models::InspectContainerData> for ContainerData {
    fn from(data: podman::models::InspectContainerData) -> Self {
        let obj: Self = glib::Object::builder()
            .property(
                "health-config",
                data.config
                    .unwrap()
                    .healthcheck
                    .map(BoxedSchema2HealthConfig),
            )
            .property(
                "health-failing-streak",
                health_failing_streak(data.state.as_ref()),
            )
            .property(
                "mounts",
                BoxedInspectMounts::from(
                    data.mounts
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|mount| match mount.name {
                            Some(ref name) => Some((name.to_owned(), mount)),
                            None => None,
                        })
                        .collect::<HashMap<_, _>>(),
                ),
            )
            .property(
                "port-bindings",
                data.host_config
                    .and_then(|config| config.port_bindings)
                    .map(BoxedPortBindings::from),
            )
            .property("size", data.size_root_fs.unwrap_or(0))
            .build();

        if let Some(logs) = data
            .state
            .and_then(|state| state.health)
            .and_then(|health| health.log)
        {
            obj.imp().health_check_log_list.sync(logs);
        }

        obj
    }
}

impl ContainerData {
    pub(crate) fn update(&self, data: podman::models::InspectContainerData) {
        self.set_health_failing_streak(health_failing_streak(data.state.as_ref()));
        if let Some(logs) = data
            .state
            .and_then(|state| state.health)
            .and_then(|health| health.log)
        {
            self.imp().health_check_log_list.sync(logs);
        }
    }

    pub(crate) fn health_check_log_list(&self) -> model::HealthCheckLogList {
        self.imp().health_check_log_list.clone()
    }

    fn set_health_failing_streak(&self, value: u32) {
        if self.health_failing_streak() == value {
            return;
        }
        self.imp().health_failing_streak.set(value);
        self.notify_health_failing_streak();
    }
}

fn health_failing_streak(state: Option<&podman::models::InspectContainerState>) -> u32 {
    state
        .and_then(|state| state.health.as_ref())
        .and_then(|results| results.failing_streak)
        .unwrap_or_default() as u32
}
