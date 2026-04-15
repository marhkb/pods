pub(crate) struct Info {
    pub(crate) arch: Option<String>,
    pub(crate) cgroup_driver: Option<String>,
    pub(crate) cgroup_version: Option<String>,
    pub(crate) hostname: Option<String>,
    pub(crate) kernel: Option<String>,
    pub(crate) mem_total: Option<u64>,
    pub(crate) cpus: Option<u16>,
    pub(crate) os: Option<String>,
    pub(crate) storage_driver: Option<String>,
    pub(crate) storage_root_dir: Option<String>,
    pub(crate) version: Option<String>,
}

impl From<bollard::plugin::SystemInfo> for Info {
    fn from(value: bollard::plugin::SystemInfo) -> Self {
        Self {
            arch: value.architecture,
            cgroup_driver: value.cgroup_driver.map(|driver| driver.to_string()),
            cgroup_version: value.cgroup_version.map(|version| version.to_string()),
            hostname: value.name,
            kernel: value.kernel_version,
            mem_total: value
                .mem_total
                .filter(|mem_total| *mem_total > 0)
                .map(|mem_total| mem_total as u64),
            cpus: value.ncpu.filter(|ncpu| *ncpu > 0).map(|ncpu| ncpu as u16),
            os: value.operating_system,
            storage_root_dir: value.docker_root_dir,
            storage_driver: value.driver,
            version: value.server_version,
        }
    }
}

impl From<podman_api::models::Info> for Info {
    fn from(value: podman_api::models::Info) -> Self {
        let (arch, cgroup_driver, cgroup_version, distribution, hostname, kernel, mem_total, cpus) =
            value
                .host
                .map(|host| {
                    (
                        host.arch,
                        host.cgroup_manager,
                        host.cgroup_version,
                        host.distribution,
                        host.hostname,
                        host.kernel,
                        host.mem_total,
                        host.cpus,
                    )
                })
                .unwrap_or_default();

        let (storage_driver, storage_root_dir) = value
            .store
            .map(|store| (store.graph_driver_name, store.graph_root))
            .unwrap_or_default();

        Self {
            arch,
            cgroup_driver,
            cgroup_version,
            hostname,
            kernel,
            mem_total: mem_total
                .filter(|mem_total| *mem_total > 0)
                .map(|mem_total| mem_total as u64),
            cpus: cpus.filter(|cpus| *cpus > 0).map(|cpus| cpus as u16),
            os: distribution.map(|distribution| {
                format!(
                    "{} (Version: {})",
                    distribution
                        .distribution
                        .as_deref()
                        .unwrap_or("Unknown distribution"),
                    distribution.version.as_deref().unwrap_or("unknown"),
                )
            }),
            storage_root_dir,
            storage_driver,
            version: value.version.and_then(|version| version.version),
        }
    }
}
