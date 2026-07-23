use gtk::glib;

use crate::engine;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ContainerRestartPolicy")]
pub(crate) enum ContainerRestartPolicy {
    Always,
    #[default]
    No,
    OnFailure,
    UnlessStopped,
}

impl From<engine::dto::RestartPolicy> for ContainerRestartPolicy {
    fn from(value: engine::dto::RestartPolicy) -> Self {
        use engine::dto::RestartPolicy::*;

        match value {
            Always => Self::Always,
            No => Self::No,
            OnFailure => Self::OnFailure,
            UnlessStopped => Self::UnlessStopped,
        }
    }
}
