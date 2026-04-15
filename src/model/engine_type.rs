use std::fmt;

use gettextrs::gettext;
use gtk::glib;

use crate::engine;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "EngineType")]
pub(crate) enum EngineType {
    Docker,
    #[default]
    Podman,
}

impl From<&engine::Engine> for EngineType {
    fn from(value: &engine::Engine) -> Self {
        match *value {
            engine::Engine::Docker(_) => Self::Docker,
            engine::Engine::Podman(_) => Self::Podman,
        }
    }
}

impl fmt::Display for EngineType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Docker => gettext("Docker"),
                Self::Podman => gettext("Podman"),
            }
        )
    }
}
