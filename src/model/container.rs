use std::cell::Cell;
use std::fmt;
use std::str::FromStr;

use gettextrs::gettext;
use gtk::glib;
use gtk::prelude::{StaticType, ToValue};
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;
use podman_api::models::{LibpodContainerInspectResponse, ListContainer};

#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ContainerStatus")]
pub enum Status {
    Configured,
    Exited,
    Running,
    Unknown,
}

impl Default for Status {
    fn default() -> Self {
        Self::Unknown
    }
}

impl FromStr for Status {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "configured" => Self::Configured,
            "exited" => Self::Exited,
            "running" => Self::Running,
            _ => return Err(()),
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
        pub(super) image_name: OnceCell<Option<String>>,
        pub(super) name: OnceCell<Option<String>>,
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
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "name",
                        "Name",
                        "The name of this container",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecEnum::new(
                        "status",
                        "Status",
                        "The status of this container",
                        Status::static_type(),
                        Status::default() as i32,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "image-name" => self.image_name.set(value.get().unwrap()).unwrap(),
                "name" => self.name.set(value.get().unwrap()).unwrap(),
                "status" => self.status.set(value.get().unwrap()),
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

impl Container {
    pub(crate) fn from_libpod(
        _list_container: ListContainer,
        inspect_response: LibpodContainerInspectResponse,
    ) -> Self {
        glib::Object::new(&[
            ("image-name", &inspect_response.image_name),
            ("name", &inspect_response.name),
            (
                "status",
                &inspect_response
                    .state
                    .and_then(|state| state.status)
                    .map_or_else(Status::default, |s| {
                        Status::from_str(&s).expect("Could not parse container status")
                    }),
            ),
        ])
        .expect("Failed to create Container")
    }
}

impl Container {
    pub(crate) fn image_name(&self) -> Option<&str> {
        self.imp().image_name.get().and_then(Option::as_deref)
    }

    pub(crate) fn name(&self) -> Option<&str> {
        self.imp().name.get().and_then(Option::as_deref)
    }

    pub(crate) fn status(&self) -> Status {
        self.imp().status.get()
    }
}
