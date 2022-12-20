use std::cell::Cell;
use std::collections::HashMap;

use gtk::glib;
use gtk::prelude::ObjectExt;
use gtk::prelude::ParamSpecBuilderExt;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::model;
use crate::monad_boxed_type;
use crate::podman;

monad_boxed_type!(pub(crate) BoxedSchema2HealthConfig(podman::models::Schema2HealthConfig) impls Debug is nullable);
monad_boxed_type!(pub(crate) BoxedPortBindings(HashMap<String, Option<Vec<podman::models::InspectHostPort>>>) impls Debug is nullable);

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct ContainerData {
        pub(super) health_config: OnceCell<Option<BoxedSchema2HealthConfig>>,
        pub(super) health_failing_streak: Cell<u32>,
        pub(super) health_check_log_list: model::HealthCheckLogList,
        pub(super) port_bindings: OnceCell<Option<BoxedPortBindings>>,
        pub(super) size: OnceCell<i64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerData {
        const NAME: &'static str = "ContainerData";
        type Type = super::ContainerData;
    }

    impl ObjectImpl for ContainerData {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecBoxed::builder::<BoxedSchema2HealthConfig>("health-config")
                        .construct_only()
                        .build(),
                    glib::ParamSpecUInt::builder("health-failing-streak")
                        .construct_only()
                        .build(),
                    glib::ParamSpecBoxed::builder::<BoxedPortBindings>("port-bindings")
                        .construct_only()
                        .build(),
                    glib::ParamSpecInt64::builder("size")
                        .construct_only()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "health-config" => self.health_config.set(value.get().unwrap()).unwrap(),
                "health-failing-streak" => {
                    self.obj().set_health_failing_streak(value.get().unwrap())
                }
                "port-bindings" => self.port_bindings.set(value.get().unwrap()).unwrap(),
                "size" => self.size.set(value.get().unwrap()).unwrap(),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "health-config" => obj.health_config().to_value(),
                "health-failing-streak" => obj.health_failing_streak().to_value(),
                "port-bindings" => obj.port_bindings().to_value(),
                "size" => obj.size().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
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

    pub(crate) fn health_config(&self) -> Option<&BoxedSchema2HealthConfig> {
        self.imp().health_config.get().unwrap().as_ref()
    }

    pub(crate) fn health_failing_streak(&self) -> u32 {
        self.imp().health_failing_streak.get()
    }

    pub(crate) fn set_health_failing_streak(&self, value: u32) {
        if self.health_failing_streak() == value {
            return;
        }
        self.imp().health_failing_streak.set(value);
        self.notify("health-failing-streak");
    }

    pub(crate) fn health_check_log_list(&self) -> model::HealthCheckLogList {
        self.imp().health_check_log_list.clone()
    }

    pub(crate) fn port_bindings(&self) -> Option<&BoxedPortBindings> {
        self.imp().port_bindings.get().unwrap().as_ref()
    }

    pub(crate) fn size(&self) -> i64 {
        *self.imp().size.get().unwrap()
    }
}

fn health_failing_streak(state: Option<&podman::models::InspectContainerState>) -> u32 {
    state
        .and_then(|state| state.health.as_ref())
        .and_then(|results| results.failing_streak)
        .unwrap_or_default() as u32
}
