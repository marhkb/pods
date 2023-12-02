#[derive(Debug, Clone, PartialEq)]
pub struct ContainerSummary {
    pub created: Option<i64>,
    pub status: Option<String>,
    pub id: Option<String>,
    pub image_id: Option<String>,
    pub image: Option<String>,
    pub names: Option<Vec<String>>,
    pub pod: Option<String>,
    pub ports: Option<Vec<PortMapping>>,
    pub state: Option<String>,
    pub started_at: Option<i64>,
    pub mounts: Option<Vec<String>>,
    pub is_infra: Option<bool>,
}

impl From<docker_api::models::ContainerSummary> for ContainerSummary {
    fn from(value: docker_api::models::ContainerSummary) -> Self {
        Self {
            created: value.created,
            status: value.status,
            id: value.id,
            image_id: value.image_id,
            image: value.image,
            names: value.names,
            pod: None,
            ports: value
                .ports
                .map(|ports| ports.into_iter().map(Into::into).collect()),
            state: value.state,
            // TODO: wrong
            started_at: Some(0),
            mounts: value.mounts.map(|mounts| {
                mounts
                    .into_iter()
                    .filter_map(|mount| mount.destination)
                    .collect()
            }),
            is_infra: Some(false),
        }
    }
}

impl From<podman_api::models::ListContainer> for ContainerSummary {
    fn from(value: podman_api::models::ListContainer) -> Self {
        Self {
            created: value.created.map(|dt| dt.timestamp()),
            status: value.status,
            id: value.id,
            image_id: value.image_id,
            image: value.image,
            names: value.names,
            pod: value.pod,
            ports: value
                .ports
                .map(|ports| ports.into_iter().map(Into::into).collect()),
            state: value.state,
            started_at: value.started_at,
            mounts: value.mounts,
            is_infra: value.is_infra,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PortMapping {
    pub container_port: Option<u16>,
    pub host_ip: Option<String>,
    pub host_port: Option<u16>,
    pub protocol: Option<String>,
    pub range: Option<u16>,
}

impl From<docker_api::models::Port> for PortMapping {
    fn from(value: docker_api::models::Port) -> Self {
        Self {
            container_port: Some(value.private_port),
            host_ip: value.ip,
            host_port: value.public_port,
            protocol: Some(value.type_),
            range: None,
        }
    }
}

impl From<podman_api::models::PortMapping> for PortMapping {
    fn from(value: podman_api::models::PortMapping) -> Self {
        Self {
            container_port: value.container_port,
            host_ip: value.host_ip,
            host_port: value.host_port,
            protocol: value.protocol,
            range: value.range,
        }
    }
}
