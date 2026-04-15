use std::fmt;

use gettextrs::gettext;
use gtk::glib;

use crate::engine;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "PodStatus")]
pub(crate) enum PodStatus {
    Created,
    Dead,
    Degraded,
    Error,
    Exited,
    Paused,
    Restarting,
    Running,
    // artificial state
    Starting,
    Stopped,
    // artificial state
    Stopping,
    #[default]
    Unknown,
}

impl PodStatus {
    pub(crate) fn is_transition(self) -> bool {
        matches!(self, Self::Restarting | Self::Starting | Self::Stopping)
    }

    pub(crate) fn can_start(self) -> bool {
        matches!(self, Self::Created | Self::Exited | Self::Stopped)
    }

    pub(crate) fn can_stop(self) -> bool {
        matches!(self, Self::Degraded | Self::Running)
    }

    pub(crate) fn can_kill(self) -> bool {
        !self.can_start()
    }

    pub(crate) fn can_restart(self) -> bool {
        matches!(self, Self::Running)
    }

    pub(crate) fn can_pause(self) -> bool {
        matches!(self, Self::Running)
    }

    pub(crate) fn can_resume(self) -> bool {
        matches!(self, Self::Paused)
    }

    pub(crate) fn can_delete(self) -> bool {
        !matches!(self, Self::Running | Self::Paused)
    }
}

impl std::cmp::Ord for PodStatus {
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

impl std::cmp::PartialOrd for PodStatus {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<engine::dto::PodStatus> for PodStatus {
    fn from(value: engine::dto::PodStatus) -> Self {
        match value {
            engine::dto::PodStatus::Created => Self::Created,
            engine::dto::PodStatus::Dead => Self::Dead,
            engine::dto::PodStatus::Degraded => Self::Degraded,
            engine::dto::PodStatus::Error => Self::Error,
            engine::dto::PodStatus::Exited => Self::Exited,
            engine::dto::PodStatus::Paused => Self::Paused,
            engine::dto::PodStatus::Restarting => Self::Restarting,
            engine::dto::PodStatus::Running => Self::Running,
            engine::dto::PodStatus::Stopped => Self::Stopped,
            engine::dto::PodStatus::Unknown => Self::Unknown,
        }
    }
}

impl fmt::Display for PodStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Created => gettext("Created"),
                Self::Dead => gettext("Dead"),
                Self::Degraded => gettext("Degraded"),
                Self::Error => gettext("Error"),
                Self::Exited => gettext("Exited"),
                Self::Paused => gettext("Paused"),
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
