use std::fmt;

use gettextrs::gettext;
use gtk::glib;

use crate::engine;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ContainerStatus")]
pub(crate) enum ContainerStatus {
    Configured,
    Created,
    Dead,
    Exited,
    Initialized,
    Paused,
    Removing,
    Restarting,
    Running,
    // artificial state
    Starting,
    Stopped,
    Stopping,
    #[default]
    Unknown,
}

impl ContainerStatus {
    pub(crate) fn is_transition(self) -> bool {
        matches!(
            self,
            Self::Removing | Self::Restarting | Self::Starting | Self::Stopping
        )
    }

    pub(crate) fn can_start(self) -> bool {
        matches!(
            self,
            Self::Configured | Self::Created | Self::Exited | Self::Initialized | Self::Stopped
        )
    }

    pub(crate) fn can_stop(self) -> bool {
        matches!(self, Self::Running | Self::Restarting)
    }

    pub(crate) fn can_kill(self) -> bool {
        matches!(
            self,
            Self::Running | Self::Stopping | Self::Restarting | Self::Paused
        )
    }

    pub(crate) fn can_restart(self) -> bool {
        !matches!(self, Self::Paused | Self::Removing | Self::Unknown)
    }

    pub(crate) fn can_pause(self) -> bool {
        self == Self::Running
    }

    pub(crate) fn can_resume(self) -> bool {
        self == Self::Paused
    }

    pub(crate) fn can_force_delete(self) -> bool {
        self != Self::Removing
    }
}

impl std::cmp::Ord for ContainerStatus {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self {
            Self::Running => {
                if let Self::Running = other {
                    std::cmp::Ordering::Equal
                } else {
                    std::cmp::Ordering::Greater
                }
            }
            Self::Paused => match other {
                Self::Running => std::cmp::Ordering::Less,
                Self::Paused => std::cmp::Ordering::Equal,
                _ => std::cmp::Ordering::Greater,
            },
            _ => match other {
                Self::Running | Self::Paused => std::cmp::Ordering::Less,
                _ => std::cmp::Ordering::Equal,
            },
        }
    }
}

impl std::cmp::PartialOrd for ContainerStatus {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<engine::dto::ContainerStatus> for ContainerStatus {
    fn from(value: engine::dto::ContainerStatus) -> Self {
        match value {
            engine::dto::ContainerStatus::Configured => Self::Configured,
            engine::dto::ContainerStatus::Created => Self::Created,
            engine::dto::ContainerStatus::Dead => Self::Dead,
            engine::dto::ContainerStatus::Exited => Self::Exited,
            engine::dto::ContainerStatus::Initialized => Self::Initialized,
            engine::dto::ContainerStatus::Paused => Self::Paused,
            engine::dto::ContainerStatus::Removing => Self::Removing,
            engine::dto::ContainerStatus::Restarting => Self::Restarting,
            engine::dto::ContainerStatus::Running => Self::Running,
            engine::dto::ContainerStatus::Stopped => Self::Stopped,
            engine::dto::ContainerStatus::Stopping => Self::Stopping,
            engine::dto::ContainerStatus::Unknown => Self::Unknown,
        }
    }
}

impl fmt::Display for ContainerStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Configured => gettext("Configured"),
                Self::Created => gettext("Created"),
                Self::Dead => gettext("Dead"),
                Self::Exited => gettext("Exited"),
                Self::Initialized => gettext("Initialized"),
                Self::Paused => gettext("Paused"),
                Self::Removing => gettext("Removing"),
                Self::Restarting => gettext("Restarting"),
                Self::Running => gettext("Running"),
                Self::Starting => gettext("Starting"),
                Self::Stopped => gettext("Stopped"),
                Self::Stopping => gettext("Stopping"),
                Self::Unknown => gettext("Unknown"),
            }
        )
    }
}
