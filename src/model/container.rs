use std::cell::{Cell, RefCell};
use std::fmt;
use std::str::FromStr;

use gettextrs::gettext;
use gtk::glib;
use gtk::prelude::{ObjectExt, StaticType, ToValue};
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use podman_api::models::{InspectContainerState, LibpodContainerInspectResponse};

#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ContainerStatus")]
pub(crate) enum Status {
    Configured,
    Exited,
    Paused,
    Running,
    Unknown,
}

impl Default for Status {
    fn default() -> Self {
        Self::Unknown
    }
}

impl FromStr for Status {
    type Err = Self;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "configured" => Self::Configured,
            "exited" => Self::Exited,
            "paused" => Self::Paused,
            "running" => Self::Running,
            _ => Self::Unknown,
        })
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Configured => gettext("Configured"),
                Self::Exited => gettext("Exited"),
                Self::Paused => gettext("Paused"),
                Self::Running => gettext("Running"),
                Self::Unknown => gettext("Unknown"),
            }
        )
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct Container {
        pub(super) image_name: RefCell<Option<String>>,
        pub(super) name: RefCell<Option<String>>,
        pub(super) status: Cell<Status>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Container {
        const NAME: &'static str = "Container";
        type Type = super::Container;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for Container {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::new(
                        "image-name",
                        "Image Name",
                        "The name of the image of this container",
                        Option::default(),
                        glib::ParamFlags::READWRITE
                            | glib::ParamFlags::CONSTRUCT
                            | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecString::new(
                        "name",
                        "Name",
                        "The name of this container",
                        Option::default(),
                        glib::ParamFlags::READWRITE
                            | glib::ParamFlags::CONSTRUCT
                            | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecEnum::new(
                        "status",
                        "Status",
                        "The status of this container",
                        Status::static_type(),
                        Status::default() as i32,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "image-name" => obj.set_image_name(value.get().unwrap()),
                "name" => obj.set_name(value.get().unwrap()),
                "status" => obj.set_status(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image-name" => obj.image_name().to_value(),
                "name" => obj.name().to_value(),
                "status" => obj.status().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct Container(ObjectSubclass<imp::Container>);
}

impl From<LibpodContainerInspectResponse> for Container {
    fn from(inspect_response: LibpodContainerInspectResponse) -> Self {
        glib::Object::new(&[
            ("image-name", &inspect_response.image_name),
            ("name", &inspect_response.name),
            ("status", &status(inspect_response.state)),
        ])
        .expect("Failed to create Container")
    }
}

impl Container {
    pub(crate) fn update(&self, inspect_response: LibpodContainerInspectResponse) {
        self.set_image_name(inspect_response.image_name);
        self.set_name(inspect_response.name);
        self.set_status(status(inspect_response.state));
    }

    pub(crate) fn image_name(&self) -> Option<String> {
        self.imp().image_name.borrow().clone()
    }

    pub(crate) fn set_image_name(&self, value: Option<String>) {
        if self.image_name() == value {
            return;
        }
        self.imp().image_name.replace(value);
        self.notify("image-name");
    }

    pub(crate) fn name(&self) -> Option<String> {
        self.imp().name.borrow().clone()
    }

    pub(crate) fn set_name(&self, value: Option<String>) {
        if self.name() == value {
            return;
        }
        self.imp().name.replace(value);
        self.notify("name");
    }

    pub(crate) fn status(&self) -> Status {
        self.imp().status.get()
    }

    pub(crate) fn set_status(&self, value: Status) {
        if self.status() == value {
            return;
        }
        self.imp().status.set(value);
        self.notify("status");
    }
}

fn status(state: Option<InspectContainerState>) -> Status {
    state
        .and_then(|state| state.status)
        .map_or_else(Status::default, |s| match Status::from_str(&s) {
            Ok(status) => status,
            Err(status) => {
                log::warn!("Unknown status: {s}");
                status
            }
        })
}
