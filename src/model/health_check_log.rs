use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::model;
use crate::podman;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct HealthCheckLog {
        pub(super) end: OnceCell<String>,
        pub(super) exit_code: OnceCell<i64>,
        pub(super) output: OnceCell<String>,
        pub(super) start: OnceCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HealthCheckLog {
        const NAME: &'static str = "HealthCheckLog";
        type Type = super::HealthCheckLog;
    }

    impl ObjectImpl for HealthCheckLog {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::builder("end")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecInt64::builder("exit-code")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecString::builder("output")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecString::builder("start")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "end" => self.end.set(value.get().unwrap()).unwrap(),
                "exit-code" => self.exit_code.set(value.get().unwrap()).unwrap(),
                "output" => self.output.set(value.get().unwrap()).unwrap(),
                "start" => self.start.set(value.get().unwrap()).unwrap(),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "end" => obj.end().to_value(),
                "exit-code" => obj.exit_code().to_value(),
                "output" => obj.output().to_value(),
                "start" => obj.start().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct HealthCheckLog(ObjectSubclass<imp::HealthCheckLog>)
        @implements gio::ListModel, model::SelectableList;
}

impl From<&podman::models::HealthCheckLog> for HealthCheckLog {
    fn from(data: &podman::models::HealthCheckLog) -> Self {
        glib::Object::builder::<Self>()
            .property("end", data.end.as_ref().unwrap())
            .property("exit-code", &data.exit_code.unwrap())
            .property("output", data.output.as_ref().unwrap())
            .property("start", data.start.as_ref().unwrap())
            .build()
    }
}

impl HealthCheckLog {
    pub(crate) fn end(&self) -> &str {
        self.imp().end.get().unwrap()
    }
    pub(crate) fn exit_code(&self) -> i64 {
        *self.imp().exit_code.get().unwrap()
    }

    pub(crate) fn output(&self) -> &str {
        self.imp().output.get().unwrap()
    }

    pub(crate) fn start(&self) -> &str {
        self.imp().start.get().unwrap()
    }
}
