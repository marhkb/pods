use std::collections::HashMap;
use std::time::Duration;

use futures::StreamExt;
use futures::TryFutureExt;
use futures::TryStreamExt;
use futures::future;
use futures::stream;
use futures::stream::BoxStream;

use crate::engine;

pub enum Containers {
    Docker(bollard::Docker),
    Podman(podman_api::api::Containers),
}

impl Containers {
    pub(crate) fn get(&self, id: impl Into<String>) -> engine::api::Container {
        match self {
            Self::Docker(docker) => engine::api::Container::Docker {
                docker: docker.to_owned(),
                id: id.into(),
            },
            Self::Podman(containers) => engine::api::Container::Podman(containers.get(id.into())),
        }
    }
}

impl Containers {
    pub(crate) async fn create(
        &self,
        opts: engine::opts::ContainerCreateOpts,
    ) -> anyhow::Result<String> {
        match self {
            Self::Docker(docker) => {
                let (opts, config) = opts.into();
                docker
                    .create_container(Some(opts), config)
                    .await
                    .map_err(anyhow::Error::from)
                    .map(|response| response.id)
            }
            Self::Podman(containers) => containers
                .create(&opts.into())
                .await
                .map_err(anyhow::Error::from)
                .map(|response| response.id),
        }
    }

    pub(crate) async fn list(&self) -> anyhow::Result<Vec<engine::dto::Container>> {
        match self {
            Self::Docker(docker) => docker
                .list_containers(Some(bollard::query_parameters::ListContainersOptions {
                    all: true,
                    ..Default::default()
                }))
                .await
                .map_err(anyhow::Error::from)
                .map(|summaries| {
                    summaries
                        .into_iter()
                        .map(engine::dto::ContainerSummary::from)
                        .map(engine::dto::Container::Summary)
                        .collect()
                }),
            Self::Podman(containers) => containers
                .list(
                    &podman_api::opts::ContainerListOpts::builder()
                        .all(true)
                        .size(false)
                        .build(),
                )
                .and_then(|summaries| {
                    stream::iter(summaries.into_iter().map(|summary| {
                        if summary
                            .mounts
                            .as_ref()
                            .filter(|mounts| !mounts.is_empty())
                            .is_some()
                        {
                            let container = containers.get(summary.id.as_ref().unwrap());

                            future::Either::Left(async move { container.inspect().await }.map_ok(
                                |inspection| engine::dto::Container::Inspection(inspection.into()),
                            ))
                        } else {
                            future::Either::Right(future::ready(Ok(
                                engine::dto::Container::Summary(summary.into()),
                            )))
                        }
                    }))
                    .buffer_unordered(20)
                    .try_collect()
                })
                .await
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn prune(
        &self,
        opts: engine::opts::ContainersPruneOpts,
    ) -> anyhow::Result<engine::dto::PruneReport> {
        match self {
            Self::Docker(docker) => docker
                .prune_containers(Some(opts.into()))
                .await
                .map_err(anyhow::Error::from)
                .map(Into::into),
            Self::Podman(containers) => containers
                .prune(&opts.into())
                .await
                .map_err(anyhow::Error::from)
                .map(Into::into),
        }
    }

    pub(crate) fn stats_stream(
        &self,
        interval: usize,
    ) -> BoxStream<'_, anyhow::Result<engine::dto::AllContainerStats>> {
        match self {
            Self::Docker(docker) => async_stream::stream! {
                let mut timer = tokio::time::interval(Duration::from_secs(interval as u64));
                timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

                let mut last_cpu_stats =
                    HashMap::<String, bollard::models::ContainerCpuStats>::new();

                loop {
                    timer.tick().await;

                    let containers = match tokio::time::timeout(
                        Duration::from_secs(5),
                        docker.list_containers(Some(
                            bollard::query_parameters::ListContainersOptions {
                                filters: Some(HashMap::from([(
                                    "status".to_owned(),
                                    vec!["running".to_owned()],
                                )])),
                                ..Default::default()
                            },
                        )),
                    )
                    .await
                    {
                        Ok(Ok(containers)) => containers,
                        _ => {
                            log::warn!("error or timeout on listing containers, retrying...");
                            continue;
                        }
                    };

                    let stats_futures = containers.into_iter().filter_map(|container| {
                        let id = container.id?;
                        let prev_cpu_stats = last_cpu_stats.remove(&id);

                        Some(async move {
                            let mut stats_stream = docker.stats(
                                &id,
                                Some(bollard::query_parameters::StatsOptions {
                                    stream: false,
                                    one_shot: true,
                                }),
                            );

                            let stats =
                                tokio::time::timeout(Duration::from_secs(3), stats_stream.next())
                                    .await;

                            match stats {
                                Ok(Some(Ok(stats))) => Ok((
                                    id,
                                    engine::dto::DockerContainerStats {
                                        prev_cpu_stats,
                                        stats,
                                    },
                                )),
                                _ => Err(anyhow::anyhow!(
                                    "error or timeout on retrieving stats for container {id}"
                                )),
                            }
                        })
                    });

                    let successful_stats = futures::future::join_all(stats_futures)
                        .await
                        .into_iter()
                        .filter_map(|r| r.inspect_err(|e| log::warn!("{e}")).ok())
                        .inspect(|(id, stats)| {
                            match stats.stats.cpu_stats {
                                Some(ref cpu) => last_cpu_stats.insert(id.clone(), cpu.clone()),
                                None => last_cpu_stats.remove(id),
                            };
                        })
                        .collect::<Vec<_>>();

                    yield Ok(engine::dto::AllContainerStats::from(successful_stats));
                }
            }
            .boxed(),
            Self::Podman(containers) => containers
                .stats_stream(
                    &podman_api::opts::ContainerStatsOpts::builder()
                        .interval(interval)
                        .build(),
                )
                .map_err(anyhow::Error::from)
                .map_ok(TryInto::try_into)
                .map(Result::flatten)
                .boxed(),
        }
    }
}
