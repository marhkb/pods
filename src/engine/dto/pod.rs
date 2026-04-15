use crate::engine;

pub(crate) enum Pod {
    Summary(engine::dto::PodSummary),
    Inspection(engine::dto::PodInspection),
}

impl Pod {
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Summary(summary) => &summary.id,
            Self::Inspection(inspection) => &inspection.summary.id,
        }
    }
}

pub(crate) struct PodSummary {
    pub(crate) created: i64,
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) status: PodStatus,
}

impl From<podman_api::models::ListPodsReport> for PodSummary {
    fn from(value: podman_api::models::ListPodsReport) -> Self {
        Self {
            created: value
                .created
                .map(|created| created.timestamp_millis())
                .unwrap_or(0),
            id: value.id.unwrap_or_default(),
            name: value.name.unwrap_or_default(),
            status: PodmanPodStatus(value.status).into(),
        }
    }
}

pub(crate) struct PodDetails {
    pub(crate) hostname: String,
}

pub(crate) struct PodInspection {
    pub(crate) summary: engine::dto::PodSummary,
    pub(crate) details: engine::dto::PodDetails,
}

impl From<podman_api::models::InspectPodData> for PodInspection {
    fn from(value: podman_api::models::InspectPodData) -> Self {
        Self {
            summary: engine::dto::PodSummary {
                created: value
                    .created
                    .map(|created| created.timestamp_millis())
                    .unwrap_or(0),
                id: value.id.unwrap_or_default(),
                name: value.name.unwrap_or_default(),
                status: PodmanPodStatus(value.state).into(),
            },
            details: engine::dto::PodDetails {
                hostname: value.hostname.unwrap_or_default(),
            },
        }
    }
}

#[derive(Default)]
pub(crate) enum PodStatus {
    Created,
    Dead,
    Degraded,
    Error,
    Exited,
    Paused,
    Restarting,
    Running,
    Stopped,
    #[default]
    Unknown,
}

struct PodmanPodStatus(Option<String>);
impl From<PodmanPodStatus> for PodStatus {
    fn from(value: PodmanPodStatus) -> Self {
        value
            .0
            .as_deref()
            .map(|status| match status {
                "Created" => Self::Created,
                "Dead" => Self::Dead,
                "Degraded" => Self::Degraded,
                "Error" => Self::Error,
                "Exited" => Self::Exited,
                "Paused" => Self::Paused,
                "Restarting" => Self::Restarting,
                "Stopped" => Self::Stopped,
                "Running" => Self::Running,
                _ => Default::default(),
            })
            .unwrap_or_default()
    }
}
