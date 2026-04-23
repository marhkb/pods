use std::collections::HashMap;
use std::fmt;

use crate::engine;

#[derive(Clone)]
pub(crate) struct ContainerCreateOpts {
    pub(crate) cmd: Option<Vec<String>>,
    pub(crate) env: HashMap<String, String>,
    pub(crate) health_config: Option<engine::dto::HealthConfig>,
    pub(crate) image: String,
    pub(crate) labels: HashMap<String, String>,
    pub(crate) memory_limit: Option<u64>,
    pub(crate) mounts: Vec<ContainerCreateMountOpts>,
    pub(crate) name: String,
    // Podman only
    pub(crate) pod: Option<String>,
    pub(crate) port_mappings: Vec<engine::dto::PortMapping>,
    // artificial option to trigger a pull before creating the container
    pub(crate) pull_latest: bool,
    // Podman only
    pub(crate) privileged: bool,
    pub(crate) terminal: bool,
    pub(crate) volumes: Vec<ContainerCreateVolumeOpts>,
}

impl Default for ContainerCreateOpts {
    fn default() -> Self {
        Self {
            cmd: None,
            env: HashMap::new(),
            health_config: None,
            image: String::new(),
            labels: HashMap::new(),
            memory_limit: None,
            mounts: Vec::new(),
            name: names::Generator::default().next().unwrap(),
            pod: None,
            port_mappings: Vec::new(),
            pull_latest: false,
            privileged: false,
            terminal: true,
            volumes: Vec::new(),
        }
    }
}

impl From<ContainerCreateOpts>
    for (
        bollard::query_parameters::CreateContainerOptions,
        bollard::plugin::ContainerCreateBody,
    )
{
    fn from(value: ContainerCreateOpts) -> Self {
        let host_config = bollard::plugin::HostConfig {
            mounts: Some(
                value
                    .mounts
                    .into_iter()
                    .map(Into::into)
                    .chain(value.volumes.into_iter().map(Into::into))
                    .collect(),
            ),
            memory: value.memory_limit.map(|memory_limit| memory_limit as i64),
            port_bindings: Some({
                value
                    .port_mappings
                    .into_iter()
                    .map(|port_mapping| {
                        (
                            format!(
                                "{}/{}",
                                port_mapping.container_port,
                                port_mapping.host_port.unwrap_or_default()
                            ),
                            bollard::plugin::PortBinding {
                                host_port: port_mapping
                                    .host_port
                                    .map(|host_port| host_port.to_string()),
                                host_ip: None,
                            },
                        )
                    })
                    .fold(HashMap::new(), |mut map, (k, v)| {
                        map.entry(k)
                            .or_insert_with(|| Some(Vec::new()))
                            .as_mut()
                            .unwrap()
                            .push(v);
                        map
                    })
            }),
            ..Default::default()
        };

        let opts = bollard::query_parameters::CreateContainerOptions {
            name: Some(value.name),
            ..Default::default()
        };

        let config = bollard::plugin::ContainerCreateBody {
            cmd: value.cmd,
            env: Some(
                value
                    .env
                    .into_iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect(),
            ),
            healthcheck: value.health_config.map(Into::into),
            host_config: Some(host_config),
            image: Some(value.image),
            labels: Some(value.labels),
            tty: Some(value.terminal),
            ..Default::default()
        };

        (opts, config)
    }
}

impl From<ContainerCreateOpts> for podman_api::opts::ContainerCreateOpts {
    fn from(value: ContainerCreateOpts) -> Self {
        let mut builder = Self::builder()
            .command(value.cmd)
            .env(value.env)
            .image(value.image)
            .labels(value.labels)
            .name(value.name)
            .mounts(value.mounts.into_iter().map(Into::into))
            .privileged(value.privileged)
            .terminal(value.terminal)
            .volumes(value.volumes.into_iter().map(Into::into));

        if let Some(health_config) = value.health_config {
            builder = builder.health_config(health_config.into());
        }

        if let Some(memory_limit) = value.memory_limit {
            builder = builder.resource_limits(podman_api::models::LinuxResources {
                block_io: None,
                cpu: None,
                devices: None,
                hugepage_limits: None,
                memory: Some(podman_api::models::LinuxMemory {
                    disable_oom_killer: None,
                    kernel: None,
                    kernel_tcp: None,
                    limit: Some(memory_limit as i64),
                    reservation: None,
                    swap: None,
                    swappiness: None,
                    use_hierarchy: None,
                }),
                network: None,
                pids: None,
                rdma: None,
                unified: None,
            });
        }

        match value.pod {
            Some(pod) => builder.pod(pod),
            None => builder.portmappings(value.port_mappings.into_iter().map(Into::into)),
        }
        .build()
    }
}

#[derive(Clone)]
pub(crate) struct ContainerCreateMountOpts {
    pub(crate) container_path: String,
    pub(crate) host_path: String,
    pub(crate) read_only: bool,
    pub(crate) selinux: SELinux,
}

impl From<ContainerCreateMountOpts> for bollard::plugin::Mount {
    fn from(value: ContainerCreateMountOpts) -> Self {
        Self {
            read_only: Some(value.read_only),
            source: Some(value.host_path),
            target: Some(value.container_path),
            typ: Some(bollard::plugin::MountTypeEnum::BIND),
            ..Default::default()
        }
    }
}

impl From<ContainerCreateMountOpts> for podman_api::models::ContainerMount {
    fn from(value: ContainerCreateMountOpts) -> Self {
        Self {
            destination: Some(value.container_path),
            source: Some(value.host_path),
            _type: Some("bind".to_owned()),
            options: Some(
                std::iter::once(if value.read_only { "ro" } else { "rw" }.to_owned())
                    .chain(if value.selinux == SELinux::NoLabel {
                        None
                    } else {
                        Some(value.selinux.to_string())
                    })
                    .collect(),
            ),
            uid_mappings: None,
            gid_mappings: None,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct ContainerCreateVolumeOpts {
    pub(crate) container_path: String,
    pub(crate) read_only: bool,
    pub(crate) selinux: SELinux,
    pub(crate) volume: String,
}

impl From<ContainerCreateVolumeOpts> for bollard::plugin::Mount {
    fn from(value: ContainerCreateVolumeOpts) -> Self {
        Self {
            read_only: Some(value.read_only),
            source: Some(value.volume),
            target: Some(value.container_path),
            typ: Some(bollard::plugin::MountTypeEnum::VOLUME),
            ..Default::default()
        }
    }
}

impl From<ContainerCreateVolumeOpts> for podman_api::models::NamedVolume {
    fn from(value: ContainerCreateVolumeOpts) -> Self {
        Self {
            dest: Some(value.container_path),
            is_anonymous: None,
            name: Some(value.volume),
            options: Some(
                std::iter::once(if value.read_only { "ro" } else { "rw" }.to_owned())
                    .chain(if value.selinux == SELinux::NoLabel {
                        None
                    } else {
                        Some(value.selinux.to_string())
                    })
                    .collect(),
            ),
        }
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub(crate) enum SELinux {
    #[default]
    NoLabel,
    Shared,
    Private,
}

impl AsRef<str> for SELinux {
    fn as_ref(&self) -> &str {
        match self {
            Self::NoLabel => "",
            Self::Shared => "z",
            Self::Private => "Z",
        }
    }
}

impl fmt::Display for SELinux {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}
