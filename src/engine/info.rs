pub(crate) struct Info {
    pub(crate) version: Option<String>,
    pub(crate) api_version: Option<String>,
    pub(crate) go_version: Option<String>,
    pub(crate) git_commit: Option<String>,
    pub(crate) built: Option<i64>,
    pub(crate) os_arch: Option<String>,
    pub(crate) cpus: Option<i64>,
}

impl From<docker_api::models::SystemInfo> for Info {
    fn from(value: docker_api::models::SystemInfo) -> Self {
        Self {
            version: value.server_version,
            api_version: value.os_version,
            go_version: None,
            git_commit: value.containerd_commit.unwrap().id,
            built: None,
            os_arch: value.architecture,
            cpus: value.ncpu.map(|ncpu| ncpu as i64),
        }
    }
}

impl From<podman_api::models::Info> for Info {
    fn from(value: podman_api::models::Info) -> Self {
        let version = value.version.unwrap();
        let host = value.host.unwrap();
        Self {
            version: version.version,
            api_version: version.api_version,
            go_version: version.go_version,
            git_commit: version.git_commit,
            built: version.built,
            os_arch: host.arch,
            cpus: host.cpus,
        }
    }
}
