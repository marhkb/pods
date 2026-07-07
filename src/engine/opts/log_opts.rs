use smart_default::SmartDefault;

#[derive(SmartDefault)]
pub(crate) struct LogsOpts {
    pub(crate) follow: bool,
    pub(crate) since: i64,
    pub(crate) stderr: bool,
    pub(crate) stdout: bool,
    #[default("all".to_string())]
    pub(crate) tail: String,
    pub(crate) timestamps: bool,
    pub(crate) until: i64,
}

impl From<LogsOpts> for bollard::query_parameters::LogsOptions {
    fn from(value: LogsOpts) -> Self {
        Self {
            follow: value.follow,
            stdout: value.stdout,
            stderr: value.stderr,
            since: value.since as i32,
            until: value.until as i32,
            timestamps: value.timestamps,
            tail: value.tail,
        }
    }
}

impl From<LogsOpts> for podman_api::opts::ContainerLogsOpts {
    fn from(value: LogsOpts) -> podman_api::opts::ContainerLogsOpts {
        podman_api::opts::ContainerLogsOpts::builder()
            .follow(value.follow)
            .since(value.since.to_string())
            .stderr(value.stderr)
            .stdout(value.stdout)
            .tail(value.tail)
            .timestamps(value.timestamps)
            .until(value.until.to_string())
            .build()
    }
}
