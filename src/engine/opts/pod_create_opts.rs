use std::collections::HashMap;

use crate::engine;

#[derive(Clone)]
pub(crate) struct PodCreateOpts {
    pub(crate) create_cmd: Option<Vec<String>>,
    pub(crate) devices: Vec<PodDevice>,
    pub(crate) hostname: String,
    pub(crate) host_management: PodHostManagement,
    pub(crate) infra: PodInfra,
    pub(crate) labels: HashMap<String, String>,
    pub(crate) name: String,
    pub(crate) port_mappings: Vec<engine::dto::PortMapping>,
}

impl From<PodCreateOpts> for podman_api::opts::PodCreateOpts {
    fn from(value: PodCreateOpts) -> Self {
        let mut builder = Self::builder()
            .hostname(value.hostname)
            .labels(value.labels)
            .name(value.name)
            .pod_create_command(value.create_cmd)
            .pod_devices(value.devices.into_iter().map(String::from))
            .portmappings(value.port_mappings.into_iter().map(Into::into));

        builder = match value.host_management {
            PodHostManagement::Containers => builder.no_manage_hosts(true),
            PodHostManagement::Pod { hosts } => {
                builder.add_hosts(hosts.into_iter().map(String::from))
            }
        };

        builder = match value.infra {
            PodInfra::Infra {
                command,
                common_pid_file,
                image,
                name,
                no_manage_resolv_conf,
            } => builder
                .infra_command(command)
                .infra_common_pid_file(common_pid_file)
                .infra_image(image)
                .infra_name(name)
                .no_manage_resolv_conf(no_manage_resolv_conf),
            PodInfra::NoInfra => builder.no_infra(true),
        };

        builder.build()
    }
}

#[derive(Clone)]
pub(crate) struct PodDevice {
    pub(crate) container_path: String,
    pub(crate) host_path: String,
    pub(crate) readable: bool,
    pub(crate) writable: bool,
    pub(crate) mknod: bool,
}

impl From<PodDevice> for String {
    fn from(value: PodDevice) -> Self {
        format!(
            "{}:{}:{}{}{}",
            value.host_path,
            value.container_path,
            if value.readable { "r" } else { "" },
            if value.writable { "w" } else { "" },
            if value.mknod { "m" } else { "" },
        )
    }
}

#[derive(Clone)]
pub(crate) enum PodHostManagement {
    Containers,
    Pod { hosts: Vec<PodHost> },
}

#[derive(Clone)]
pub(crate) struct PodHost {
    pub(crate) ip: String,
    pub(crate) name: String,
}

impl From<PodHost> for String {
    fn from(value: PodHost) -> Self {
        format!("{}:{}", value.name, value.ip)
    }
}

#[derive(Clone)]
pub(crate) enum PodInfra {
    Infra {
        command: Option<Vec<String>>,
        common_pid_file: Option<String>,
        image: Option<String>,
        name: Option<String>,
        no_manage_resolv_conf: bool,
    },
    NoInfra,
}
