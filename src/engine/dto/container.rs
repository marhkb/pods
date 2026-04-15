use std::collections::HashMap;

use gtk::glib;

use crate::engine;

pub(crate) enum Container {
    Summary(engine::dto::ContainerSummary),
    Inspection(engine::dto::ContainerInspection),
}

impl Container {
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Summary(dto) => &dto.id,
            Self::Inspection(dto) => &dto.summary.id,
        }
    }
}

pub(crate) struct ContainerSummary {
    pub(crate) created: i64,
    pub(crate) health_status: engine::dto::HealthStatus,
    pub(crate) id: String,
    pub(crate) image_id: String,
    pub(crate) image_name: Option<String>,
    pub(crate) is_infra: bool,
    pub(crate) mounts: Vec<engine::dto::Mount>,
    pub(crate) name: String,
    pub(crate) pod_id: Option<String>,
    pub(crate) ports: Vec<engine::dto::PortMapping>,
    pub(crate) status: engine::dto::ContainerStatus,
}

impl From<bollard::plugin::ContainerSummary> for ContainerSummary {
    fn from(value: bollard::plugin::ContainerSummary) -> Self {
        Self {
            created: value.created.unwrap_or_default(),
            health_status: value.health.and_then(|health| health.status).into(),
            id: value.id.unwrap(),
            image_id: value.image_id.unwrap_or_default(),
            image_name: value.image.filter(|name| !name.is_empty()),
            is_infra: false,
            mounts: value
                .mounts
                .map(|mounts| mounts.into_iter().map(Into::into).collect())
                .unwrap_or_default(),
            name: value
                .names
                .and_then(|mut names| {
                    if names.is_empty() {
                        None
                    } else {
                        Some(names.swap_remove(0))
                    }
                })
                .map(|mut name| {
                    if !name.is_empty() {
                        name.drain(..1);
                    }
                    name
                })
                .unwrap_or_default(),
            pod_id: None,
            ports: value
                .ports
                .map(|ports| ports.into_iter().map(Into::into).collect())
                .unwrap_or_default(),
            status: value.state.into(),
        }
    }
}

impl From<podman_api::models::ListContainer> for ContainerSummary {
    fn from(value: podman_api::models::ListContainer) -> Self {
        Self {
            created: value
                .created
                .map(|date_time| date_time.timestamp())
                .unwrap_or(0),
            health_status: value.status.as_deref().into(),
            id: value.id.unwrap(),
            image_id: value.image_id.unwrap_or_default(),
            image_name: value.image.filter(|name| !name.is_empty()),
            is_infra: value.is_infra.unwrap_or(false),
            // mounts are missing in a podman summary
            mounts: Vec::new(),
            name: value
                .names
                .and_then(|mut names| {
                    if names.is_empty() {
                        None
                    } else {
                        Some(names.swap_remove(0))
                    }
                })
                .unwrap_or_default(),
            pod_id: value.pod,
            ports: value
                .ports
                .map(|ports| ports.into_iter().map(Into::into).collect())
                .unwrap_or_default(),
            status: PodmanContainerStatus(value.state).into(),
        }
    }
}

pub(crate) struct ContainerDetails {
    pub(crate) health_config: Option<HealthConfig>,
    pub(crate) health_failing_streak: u32,
    pub(crate) health_check_logs: Vec<HealthCheckLog>,
    pub(crate) size: i64,
    pub(crate) up_since: i64,
}

pub(crate) struct ContainerInspection {
    pub(crate) summary: engine::dto::ContainerSummary,
    pub(crate) details: engine::dto::ContainerDetails,
}

impl ContainerInspection {
    pub(crate) fn from_docker(
        image_name: Option<String>,
        inspection: bollard::plugin::ContainerInspectResponse,
    ) -> Self {
        let (status, up_since, health_info) = inspection
            .state
            .map(|state| {
                (
                    state.status,
                    state.started_at,
                    state
                        .health
                        .map(|health| (health.failing_streak, health.log, health.status)),
                )
            })
            .unwrap_or_default();

        let (health_failing_streak, health_check_logs, health_status) = health_info
            .map(|(failing_streak, log, status)| {
                (
                    failing_streak.unwrap_or(0) as u32,
                    log.unwrap_or_default(),
                    status,
                )
            })
            .unwrap_or_default();

        Self {
            summary: engine::dto::ContainerSummary {
                created: inspection
                    .created
                    .and_then(|created| glib::DateTime::from_iso8601(&created, None).ok())
                    .map(|date_time| date_time.to_unix())
                    .unwrap_or(0),
                health_status: health_status.into(),
                id: inspection.id.unwrap(),
                image_id: inspection.image.unwrap_or_default(),
                image_name: image_name.filter(|name| !name.is_empty()),
                is_infra: false,
                mounts: inspection
                    .mounts
                    .map(|mounts| mounts.into_iter().map(Into::into).collect())
                    .unwrap_or_default(),
                name: inspection
                    .name
                    .map(|mut name| {
                        if !name.is_empty() {
                            name.drain(..1);
                        }
                        name
                    })
                    .unwrap_or_default(),
                pod_id: None,
                ports: inspection
                    .network_settings
                    .and_then(|settings| settings.ports)
                    .map(PortMappings::from)
                    .map(PortMappings::into_inner)
                    .unwrap_or_default(),
                status: status.into(),
            },
            details: engine::dto::ContainerDetails {
                health_config: inspection
                    .config
                    .and_then(|c| c.healthcheck)
                    .map(Into::into),
                health_failing_streak,
                health_check_logs: health_check_logs.into_iter().map(Into::into).collect(),
                size: inspection.size_root_fs.unwrap_or(0) + inspection.size_rw.unwrap_or(0),
                up_since: up_since
                    .and_then(|up_since| glib::DateTime::from_iso8601(&up_since, None).ok())
                    .map(|date_time| date_time.to_unix())
                    .unwrap_or(0),
            },
        }
    }
}

impl From<podman_api::models::ContainerInspectResponseLibpod> for ContainerInspection {
    fn from(value: podman_api::models::ContainerInspectResponseLibpod) -> Self {
        let (status, up_since, health_info) = value
            .state
            .map(|state| {
                (
                    state.status,
                    state.started_at,
                    state
                        .health
                        .map(|health| (health.failing_streak, health.log, health.status)),
                )
            })
            .unwrap_or_default();

        let (health_failing_streak, health_check_logs, health_status) = health_info
            .map(|(failing_streak, log, status)| {
                (
                    failing_streak.unwrap_or(0) as u32,
                    log.unwrap_or_default(),
                    status,
                )
            })
            .unwrap_or_default();

        Self {
            summary: engine::dto::ContainerSummary {
                created: value
                    .created
                    .map(|date_time| date_time.timestamp())
                    .unwrap_or(0),
                health_status: health_status.as_deref().into(),
                id: value.id.unwrap(),
                image_id: value.image.unwrap_or_default(),
                image_name: value.image_name.filter(|name| !name.is_empty()),
                is_infra: value.is_infra.unwrap_or(false),
                mounts: value
                    .mounts
                    .map(|mounts| mounts.into_iter().map(Into::into).collect())
                    .unwrap_or_default(),
                name: value.name.unwrap_or_default(),
                pod_id: value.pod,
                ports: value
                    .network_settings
                    .and_then(|settings| settings.ports)
                    .map(PortMappings::from)
                    .map(PortMappings::into_inner)
                    .unwrap_or_default(),
                status: PodmanContainerStatus(status).into(),
            },
            details: engine::dto::ContainerDetails {
                health_config: value
                    .config
                    .and_then(|config| config.healthcheck)
                    .map(Into::into),
                health_failing_streak,
                health_check_logs: health_check_logs.into_iter().map(Into::into).collect(),
                size: value.size_root_fs.unwrap_or(0) + value.size_rw.unwrap_or(0),
                up_since: up_since.map(|date_time| date_time.timestamp()).unwrap_or(0),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
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
    Stopped,
    Stopping,
    #[default]
    Unknown,
}

impl From<Option<bollard::plugin::ContainerStateStatusEnum>> for ContainerStatus {
    fn from(value: Option<bollard::plugin::ContainerStateStatusEnum>) -> Self {
        value
            .map(|value| match value {
                bollard::plugin::ContainerStateStatusEnum::CREATED => Self::Created,
                bollard::plugin::ContainerStateStatusEnum::DEAD => Self::Dead,
                bollard::plugin::ContainerStateStatusEnum::EXITED => Self::Exited,
                bollard::plugin::ContainerStateStatusEnum::PAUSED => Self::Paused,
                bollard::plugin::ContainerStateStatusEnum::REMOVING => Self::Removing,
                bollard::plugin::ContainerStateStatusEnum::RESTARTING => Self::Restarting,
                bollard::plugin::ContainerStateStatusEnum::RUNNING => Self::Running,
                bollard::plugin::ContainerStateStatusEnum::EMPTY => Self::default(),
            })
            .unwrap_or_default()
    }
}

impl From<Option<bollard::plugin::ContainerSummaryStateEnum>> for ContainerStatus {
    fn from(value: Option<bollard::plugin::ContainerSummaryStateEnum>) -> Self {
        value
            .map(|value| match value {
                bollard::plugin::ContainerSummaryStateEnum::CREATED => Self::Created,
                bollard::plugin::ContainerSummaryStateEnum::DEAD => Self::Dead,
                bollard::plugin::ContainerSummaryStateEnum::EXITED => Self::Exited,
                bollard::plugin::ContainerSummaryStateEnum::PAUSED => Self::Paused,
                bollard::plugin::ContainerSummaryStateEnum::REMOVING => Self::Removing,
                bollard::plugin::ContainerSummaryStateEnum::RESTARTING => Self::Restarting,
                bollard::plugin::ContainerSummaryStateEnum::RUNNING => Self::Running,
                bollard::plugin::ContainerSummaryStateEnum::EMPTY => Self::default(),
            })
            .unwrap_or_default()
    }
}

struct PodmanContainerStatus(Option<String>);
impl From<PodmanContainerStatus> for ContainerStatus {
    fn from(value: PodmanContainerStatus) -> Self {
        value
            .0
            .as_deref()
            .map(|value| match value {
                "configured" => Self::Configured,
                "created" => Self::Created,
                "dead" => Self::Dead,
                "exited" => Self::Exited,
                "initialized" => Self::Initialized,
                "paused" => Self::Paused,
                "removing" => Self::Removing,
                "restarting" => Self::Restarting,
                "running" => Self::Running,
                "stopped" => Self::Stopped,
                "stopping" => Self::Stopping,
                _ => Self::default(),
            })
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HealthConfig {
    pub(crate) interval: Option<i64>,
    pub(crate) retries: Option<i64>,
    pub(crate) start_period: Option<i64>,
    pub(crate) test: Option<Vec<String>>,
    pub(crate) timeout: Option<i64>,
}

impl From<bollard::plugin::HealthConfig> for HealthConfig {
    fn from(value: bollard::plugin::HealthConfig) -> Self {
        Self {
            interval: value.interval,
            retries: value.retries,
            start_period: value.start_period,
            test: value.test,
            timeout: value.timeout,
        }
    }
}

impl From<HealthConfig> for bollard::plugin::HealthConfig {
    fn from(value: HealthConfig) -> Self {
        Self {
            interval: value.interval,
            retries: value.retries,
            start_interval: None,
            start_period: value.start_period,
            test: value.test,
            timeout: value.timeout,
        }
    }
}

impl From<podman_api::models::Schema2HealthConfig> for HealthConfig {
    fn from(value: podman_api::models::Schema2HealthConfig) -> Self {
        Self {
            interval: value.interval,
            retries: value.retries,
            start_period: value.start_period,
            test: value.test,
            timeout: value.timeout,
        }
    }
}

impl From<HealthConfig> for podman_api::models::Schema2HealthConfig {
    fn from(value: HealthConfig) -> Self {
        Self {
            interval: value.interval,
            retries: value.retries,
            start_period: value.start_period,
            test: value.test,
            timeout: value.timeout,
        }
    }
}

#[derive(Debug)]
pub(crate) struct HealthCheckLog {
    pub(crate) end: Option<String>,
    pub(crate) exit_code: Option<i64>,
    pub(crate) output: Option<String>,
    pub(crate) start: Option<String>,
}

impl From<bollard::plugin::HealthcheckResult> for HealthCheckLog {
    fn from(value: bollard::plugin::HealthcheckResult) -> Self {
        Self {
            end: value.end,
            exit_code: value.exit_code,
            output: value.output,
            start: value.start,
        }
    }
}

impl From<podman_api::models::HealthCheckLog> for HealthCheckLog {
    fn from(value: podman_api::models::HealthCheckLog) -> Self {
        Self {
            end: value.end,
            exit_code: value.exit_code,
            output: value.output,
            start: value.start,
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub(crate) enum HealthStatus {
    Healthy,
    Starting,
    #[default]
    Unconfigured,
    Unhealthy,
}

impl From<Option<bollard::plugin::HealthStatusEnum>> for HealthStatus {
    fn from(value: Option<bollard::plugin::HealthStatusEnum>) -> Self {
        value
            .map(|value| match value {
                bollard::plugin::HealthStatusEnum::HEALTHY => Self::Healthy,
                bollard::plugin::HealthStatusEnum::STARTING => Self::Starting,
                bollard::plugin::HealthStatusEnum::NONE => Self::Unconfigured,
                bollard::plugin::HealthStatusEnum::UNHEALTHY => Self::Unhealthy,
                bollard::plugin::HealthStatusEnum::EMPTY => Self::default(),
            })
            .unwrap_or_default()
    }
}

impl From<Option<bollard::plugin::ContainerSummaryHealthStatusEnum>> for HealthStatus {
    fn from(value: Option<bollard::plugin::ContainerSummaryHealthStatusEnum>) -> Self {
        value
            .map(|value| match value {
                bollard::plugin::ContainerSummaryHealthStatusEnum::HEALTHY => Self::Healthy,
                bollard::plugin::ContainerSummaryHealthStatusEnum::STARTING => Self::Starting,
                bollard::plugin::ContainerSummaryHealthStatusEnum::NONE => Self::Unconfigured,
                bollard::plugin::ContainerSummaryHealthStatusEnum::UNHEALTHY => Self::Unhealthy,
                bollard::plugin::ContainerSummaryHealthStatusEnum::EMPTY => Self::default(),
            })
            .unwrap_or_default()
    }
}

impl From<Option<&str>> for HealthStatus {
    fn from(value: Option<&str>) -> Self {
        value
            .map(|value| match value {
                "healthy" => Self::Healthy,
                "starting" => Self::Starting,
                "none" | "" => Self::Unconfigured,
                "unhealthy" => Self::Unhealthy,
                _ => Self::default(),
            })
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Mount {
    pub(crate) destination: String,
    pub(crate) mode: String,
    pub(crate) name: String,
    pub(crate) rw: bool,
}

impl From<bollard::plugin::MountPoint> for Mount {
    fn from(value: bollard::plugin::MountPoint) -> Self {
        Self {
            destination: value.destination.unwrap(),
            mode: value.mode.unwrap_or_default(),
            name: value.name.unwrap_or_default(),
            rw: value.rw.unwrap_or_default(),
        }
    }
}

impl From<podman_api::models::InspectMount> for Mount {
    fn from(value: podman_api::models::InspectMount) -> Self {
        Self {
            destination: value.destination.unwrap_or_default(),
            mode: value.mode.unwrap_or_default(),
            name: value.name.unwrap_or_default(),
            rw: value.rw.unwrap_or_default(),
        }
    }
}

pub(crate) struct PortMappings(Vec<engine::dto::PortMapping>);

impl PortMappings {
    pub(crate) fn into_inner(self) -> Vec<engine::dto::PortMapping> {
        self.0
    }
}

impl From<bollard::plugin::PortMap> for PortMappings {
    fn from(value: bollard::plugin::PortMap) -> Self {
        Self(
            value
                .into_iter()
                .flat_map(|(key, bindings)| {
                    let (container_port, protocol) = key.split_once('/').unwrap();

                    bindings.map(|bindings| {
                        bindings
                            .into_iter()
                            .map(|binding| engine::dto::PortMapping {
                                container_port: container_port.parse().unwrap(),
                                host_ip: binding.host_ip.unwrap_or_default(),
                                host_port: binding.host_port.map(|port| port.parse().unwrap()),
                                protocol: protocol.into(),
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .flatten()
                .collect(),
        )
    }
}

type PodmanPortMap = HashMap<String, Option<Vec<podman_api::models::InspectHostPort>>>;
impl From<PodmanPortMap> for PortMappings {
    fn from(value: PodmanPortMap) -> Self {
        Self(
            value
                .into_iter()
                .flat_map(|(key, bindings)| {
                    let (container_port, protocol) = key.split_once('/').unwrap();

                    bindings.map(|bindings| {
                        bindings
                            .into_iter()
                            .map(|binding| engine::dto::PortMapping {
                                container_port: container_port.parse().unwrap_or_default(),
                                host_ip: binding.host_ip.unwrap_or_default(),
                                host_port: binding
                                    .host_port
                                    .map(|port| port.parse().unwrap_or_default()),
                                protocol: protocol.into(),
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .flatten()
                .collect(),
        )
    }
}
