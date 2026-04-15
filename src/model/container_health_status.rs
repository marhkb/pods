use std::fmt;

use gettextrs::gettext;
use gtk::glib;

use crate::engine;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ContainerHealthStatus")]
pub(crate) enum ContainerHealthStatus {
    Healthy,
    Starting,
    #[default]
    Unconfigured,
    Unhealthy,
}

impl From<engine::dto::HealthStatus> for ContainerHealthStatus {
    fn from(value: engine::dto::HealthStatus) -> Self {
        match value {
            engine::dto::HealthStatus::Healthy => Self::Healthy,
            engine::dto::HealthStatus::Starting => Self::Starting,
            engine::dto::HealthStatus::Unconfigured => Self::Unconfigured,
            engine::dto::HealthStatus::Unhealthy => Self::Unhealthy,
        }
    }
}

impl fmt::Display for ContainerHealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Healthy => gettext("Healthy"),
                Self::Starting => gettext("Checking"),
                Self::Unconfigured => gettext("Unconfigured"),
                Self::Unhealthy => gettext("Unhealthy"),
            }
        )
    }
}
