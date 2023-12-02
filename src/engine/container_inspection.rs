#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ContainerInspection {
    // pub healthcheck: Option<Schema2HealthConfig>,
    // pub state: Option<InspectContainerState>,
    // pub mounts: Option<Vec<InspectMount>>,
    // pub host_config: Option<InspectContainerHostConfig>,
}

impl From<docker_api::models::ContainerInspect200Response> for ContainerInspection {
    fn from(value: docker_api::models::ContainerInspect200Response) -> Self {
        // Self {
        //
        // }
        todo!()
    }
}

impl From<podman_api::models::InspectContainerData> for ContainerInspection {
    fn from(value: podman_api::models::InspectContainerData) -> Self {
        // Self {

        // }
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PortMapping {
    pub container_port: Option<u16>,
    pub host_port: Option<u16>,
    pub protocol: Option<String>,
    pub range: Option<u16>,
}

impl From<docker_api::models::Port> for PortMapping {
    fn from(value: docker_api::models::Port) -> Self {
        Self {
            container_port: Some(value.private_port),
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
            host_port: value.host_port,
            protocol: value.protocol,
            range: value.range,
        }
    }
}
