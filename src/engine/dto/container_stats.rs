#[derive(Debug)]
pub(crate) struct AllContainerStats(Vec<(String, ContainerStats)>);

impl AllContainerStats {
    pub(crate) fn into_single_stats(self) -> Vec<(String, ContainerStats)> {
        self.0
    }
}

pub(crate) struct DockerContainerStats {
    pub(crate) prev_cpu_stats: Option<bollard::plugin::ContainerCpuStats>,
    pub(crate) stats: bollard::plugin::ContainerStatsResponse,
}

impl From<Vec<(String, DockerContainerStats)>> for AllContainerStats {
    fn from(value: Vec<(String, DockerContainerStats)>) -> Self {
        Self(
            value
                .into_iter()
                .map(|(id, stats)| (id, stats.into()))
                .collect(),
        )
    }
}

impl TryFrom<podman_api::models::ContainerStats200Response> for AllContainerStats {
    type Error = anyhow::Error;

    fn try_from(
        mut value: podman_api::models::ContainerStats200Response,
    ) -> Result<Self, Self::Error> {
        value
            .as_object_mut()
            .and_then(|object| object.remove("Stats"))
            .ok_or_else(|| anyhow::anyhow!("Field 'Stats' is not present"))
            .and_then(|value| {
                serde_json::from_value::<Vec<podman_api::models::ContainerStats>>(value)
                    .map_err(anyhow::Error::from)
            })
            .map(|stats| {
                stats
                    .into_iter()
                    .map(|stats| {
                        (
                            stats.container_id.clone().unwrap(),
                            ContainerStats::from(stats),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .map(Self)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ContainerStats {
    pub(crate) cpu: Option<f64>,
    pub(crate) mem_usage: Option<u64>,
    pub(crate) mem_limit: Option<u64>,
    pub(crate) mem_perc: Option<f64>,
    pub(crate) net_input: Option<u64>,
    pub(crate) net_output: Option<u64>,
    pub(crate) block_input: Option<u64>,
    pub(crate) block_output: Option<u64>,
}

impl From<DockerContainerStats> for ContainerStats {
    fn from(value: DockerContainerStats) -> Self {
        let DockerContainerStats {
            prev_cpu_stats,
            stats,
        } = value;

        let net_io = stats.networks.map(|networks| {
            networks
                .values()
                .fold((0, 0), |(net_input, net_output), stats| {
                    (
                        net_input + stats.rx_bytes.unwrap_or(0),
                        net_output + stats.tx_bytes.unwrap_or(0),
                    )
                })
        });

        let block_io = stats
            .blkio_stats
            .and_then(|stats| stats.io_service_bytes_recursive)
            .map(|blkios| {
                blkios.into_iter().fold((0, 0), |(blkin, blkout), blkio| {
                    match blkio.op.as_deref() {
                        Some("read") => (blkin + blkio.value.unwrap_or_default(), blkout),
                        Some("write") => (blkin, blkout + blkio.value.unwrap_or_default()),
                        _ => (blkin, blkout),
                    }
                })
            });

        let (mem_usage, mem_limit) = stats
            .memory_stats
            .map(|stats| (stats.usage, stats.limit))
            .unwrap_or_default();

        Self {
            cpu: match (stats.cpu_stats, prev_cpu_stats) {
                (Some(cpu_stats), Some(prev_cpu_stats)) => {
                    let cpu_usage = cpu_stats
                        .cpu_usage
                        .and_then(|cpu_usage| cpu_usage.total_usage)
                        .unwrap_or(0) as f64;
                    let pre_cpu_usage = prev_cpu_stats
                        .cpu_usage
                        .and_then(|cpu_usage| cpu_usage.total_usage)
                        .unwrap_or(0) as f64;

                    let system_usage = cpu_stats.system_cpu_usage.unwrap_or_default() as f64;
                    let pre_system_usage =
                        prev_cpu_stats.system_cpu_usage.unwrap_or_default() as f64;

                    let cpu_delta = cpu_usage - pre_cpu_usage;
                    let system_delta = system_usage - pre_system_usage;

                    let online_cpus = cpu_stats
                        .online_cpus
                        .map(|online_cpus| online_cpus as f64)
                        .unwrap_or(1.0);

                    Some(((cpu_delta / system_delta) * online_cpus * 100.0).max(0.0))
                }
                _ => None,
            },
            mem_perc: match (mem_usage, mem_limit) {
                (Some(mem_usage), Some(mem_limit)) => Some(mem_usage as f64 / mem_limit as f64),
                _ => None,
            },
            mem_usage,
            mem_limit,
            net_input: net_io.map(|(net_input, _)| net_input),
            net_output: net_io.map(|(_, net_output)| net_output),
            block_input: block_io.map(|(block_input, _)| block_input),
            block_output: block_io.map(|(_, block_output)| block_output),
        }
    }
}

impl From<podman_api::models::ContainerStats> for ContainerStats {
    fn from(value: podman_api::models::ContainerStats) -> Self {
        Self {
            cpu: value.cpu,
            mem_usage: value.mem_usage,
            mem_limit: value.mem_limit,
            mem_perc: value.mem_perc,
            net_input: value.net_input,
            net_output: value.net_output,
            block_input: value.block_input,
            block_output: value.block_output,
        }
    }
}
