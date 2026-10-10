use std::fmt;

#[derive(Clone, Debug)]
pub(crate) struct PortBinding {
    pub(crate) host_ip: String,
    pub(crate) host_port: Option<u16>,
    pub(crate) protocol: PortMappingProtocol,
    pub(crate) target_port: u16,
}

impl From<bollard::plugin::PortSummary> for PortBinding {
    fn from(value: bollard::plugin::PortSummary) -> Self {
        Self {
            host_ip: value.ip.unwrap_or_default(),
            host_port: value.public_port,
            protocol: value.typ.into(),
            target_port: value.private_port,
        }
    }
}

impl From<podman_api::models::PortMapping> for PortBinding {
    fn from(value: podman_api::models::PortMapping) -> Self {
        Self {
            host_ip: value.host_ip.unwrap_or_default(),
            host_port: value.host_port,
            protocol: value.protocol.as_deref().into(),
            target_port: value.container_port.unwrap_or(0),
        }
    }
}

impl From<PortBinding> for podman_api::models::PortMapping {
    fn from(value: PortBinding) -> Self {
        Self {
            container_port: Some(value.target_port),
            host_ip: None,
            host_port: value.host_port,
            protocol: Some(value.protocol.to_string()),
            range: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) enum PortMappingProtocol {
    #[default]
    Tcp,
    Udp,
    Sctp,
}

impl From<Option<bollard::plugin::PortSummaryTypeEnum>> for PortMappingProtocol {
    fn from(value: Option<bollard::plugin::PortSummaryTypeEnum>) -> Self {
        value
            .map(|protocol| match protocol {
                bollard::plugin::PortSummaryTypeEnum::TCP => Self::Tcp,
                bollard::plugin::PortSummaryTypeEnum::UDP => Self::Udp,
                bollard::plugin::PortSummaryTypeEnum::SCTP => Self::Sctp,
                bollard::plugin::PortSummaryTypeEnum::EMPTY => Default::default(),
            })
            .unwrap_or_default()
    }
}

impl From<&str> for PortMappingProtocol {
    fn from(value: &str) -> Self {
        match value {
            "tcp" => Self::Tcp,
            "udp" => Self::Udp,
            "sctp" => Self::Sctp,
            _ => Default::default(),
        }
    }
}

impl From<Option<&str>> for PortMappingProtocol {
    fn from(value: Option<&str>) -> Self {
        value.map(Into::into).unwrap_or_default()
    }
}

impl fmt::Display for PortMappingProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Tcp => "tcp",
                Self::Udp => "udp",
                Self::Sctp => "sctp",
            }
        )
    }
}
