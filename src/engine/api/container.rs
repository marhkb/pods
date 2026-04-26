use std::collections::HashMap;
use std::time::Duration;

use bytes::Bytes;
use futures::StreamExt;
use futures::TryStreamExt;
use futures::stream::BoxStream;
use http_body_util::Either;
use http_body_util::Full;

use crate::engine;
use crate::engine::dto::ContainerInspection;

pub(crate) enum Container {
    Docker { docker: bollard::Docker, id: String },
    Podman(podman_api::api::Container),
}

impl Container {
    pub(crate) fn id(&self) -> &str {
        match &self {
            Self::Docker { id, .. } => id,
            Self::Podman(container) => container.id().as_ref(),
        }
    }
}

impl Container {
    pub(crate) async fn commit(
        &self,
        opts: engine::opts::ContainerCommitOpts,
    ) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => docker
                .commit_container(
                    bollard::query_parameters::CommitContainerOptions {
                        container: Some(id.to_owned()),
                        ..opts.into()
                    },
                    bollard::models::ContainerConfig::default(),
                )
                .await
                .map_err(anyhow::Error::from)
                .map(|_| ()),
            Self::Podman(container) => container
                .commit(&opts.into())
                .await
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) fn copy_from(&self, path: String) -> BoxStream<'_, anyhow::Result<Bytes>> {
        match self {
            Self::Docker { docker, id } => docker
                .download_from_container(
                    id,
                    Some(bollard::query_parameters::DownloadFromContainerOptions { path }),
                )
                .map_err(anyhow::Error::from)
                .boxed(),

            Self::Podman(container) => container
                .copy_from(path)
                .map_err(anyhow::Error::from)
                .map_ok(Into::into)
                .boxed(),
        }
    }

    pub(crate) async fn copy_to(&self, path: String, buf: Vec<u8>) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => docker
                .upload_to_container(
                    id,
                    Some(bollard::query_parameters::UploadToContainerOptions {
                        path,
                        ..Default::default()
                    }),
                    Either::Left(Full::new(buf.into())),
                )
                .await
                .map_err(anyhow::Error::from),

            Self::Podman(container) => container
                .copy_to(path, buf.into())
                .await
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn create_exec(
        &self,
        opts: engine::opts::ExecCreateOpts,
    ) -> anyhow::Result<engine::api::Exec> {
        match self {
            Self::Docker { docker, id } => docker
                .create_exec(id, opts)
                .await
                .map_err(anyhow::Error::from)
                .map(|res| engine::api::Exec::Docker {
                    docker: docker.clone(),
                    id: res.id,
                }),

            Self::Podman(container) => container
                .create_exec(&opts.into())
                .await
                .map_err(anyhow::Error::from)
                .map(engine::api::Exec::Podman),
        }
    }

    pub(crate) async fn generate_kube_yaml(&self, service: bool) -> anyhow::Result<String> {
        match self {
            Self::Docker { .. } => {
                anyhow::bail!("kube generation is not supported by the Docker API")
            }
            Self::Podman(container) => container
                .generate_kube_yaml(service)
                .await
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn healthcheck(&self) -> anyhow::Result<()> {
        match self {
            Self::Docker { .. } => {
                anyhow::bail!("manual health checks are not supported by the Docker API")
            }
            Self::Podman(container) => container
                .healthcheck()
                .await
                .map_err(anyhow::Error::from)
                .map(|_| ()),
        }
    }

    pub(crate) async fn inspect(&self) -> anyhow::Result<engine::dto::ContainerInspection> {
        match self {
            Self::Docker { docker, id } => {
                let mut summaries = docker
                    .list_containers(Some(bollard::query_parameters::ListContainersOptions {
                        all: true,
                        filters: Some(HashMap::from([("id".to_owned(), vec![id.clone()])])),
                        ..Default::default()
                    }))
                    .await?;

                let summary = summaries
                    .pop()
                    .ok_or_else(|| anyhow::anyhow!("container was not found"))?;

                docker
                    .inspect_container(
                        id,
                        Some(bollard::query_parameters::InspectContainerOptions { size: true }),
                    )
                    .await
                    .map_err(anyhow::Error::from)
                    .map(|inspection| ContainerInspection::from_docker(summary.image, inspection))
            }
            Self::Podman(container) => container
                .inspect()
                .await
                .map_err(anyhow::Error::from)
                .map(Into::into),
        }
    }

    pub(crate) async fn json(&self) -> anyhow::Result<String> {
        match self {
            Self::Docker { docker, id } => docker
                .inspect_container(
                    id,
                    Some(bollard::query_parameters::InspectContainerOptions { size: true }),
                )
                .await
                .map_err(anyhow::Error::from)
                .and_then(|response| {
                    serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                }),

            Self::Podman(container) => container
                .inspect()
                .await
                .map_err(anyhow::Error::from)
                .and_then(|response| {
                    serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                }),
        }
    }

    pub(crate) async fn kill(&self) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => docker
                .kill_container(id, None)
                .await
                .map_err(anyhow::Error::from),
            Self::Podman(container) => container.kill().await.map_err(anyhow::Error::from),
        }
    }

    pub(crate) fn logs(
        &self,
        opts: engine::opts::LogsOpts,
    ) -> BoxStream<'_, anyhow::Result<engine::conn::TtyChunk>> {
        match self {
            Self::Docker { docker, id } => docker
                .logs(id, Some(opts.into()))
                .map_err(anyhow::Error::from)
                .map_ok(Into::into)
                .boxed(),
            Self::Podman(container) => container
                .logs(&opts.into())
                .map_err(anyhow::Error::from)
                .map_ok(Into::into)
                .boxed(),
        }
    }

    pub(crate) async fn pause(&self) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => docker
                .pause_container(id)
                .await
                .map_err(anyhow::Error::from),
            Self::Podman(container) => container.pause().await.map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn remove(&self, force: bool) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => docker
                .remove_container(
                    id,
                    Some(bollard::query_parameters::RemoveContainerOptions {
                        force,
                        ..Default::default()
                    }),
                )
                .await
                .map_err(anyhow::Error::from),

            Self::Podman(container) => container
                .delete(
                    &podman_api::opts::ContainerDeleteOpts::builder()
                        .force(force)
                        .build(),
                )
                .await
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn start(&self) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => docker
                .start_container(id, None)
                .await
                .map_err(anyhow::Error::from),
            Self::Podman(container) => container.start(None).await.map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn stop(&self) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => docker
                .stop_container(id, None)
                .await
                .map_err(anyhow::Error::from),
            Self::Podman(container) => container
                .stop(&Default::default())
                .await
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) fn top_stream(
        &self,
        delay: usize,
    ) -> BoxStream<'_, anyhow::Result<engine::dto::Top>> {
        match self {
            Self::Docker { docker, id } => async_stream::stream! {
                let mut timer = tokio::time::interval(Duration::from_secs(delay as u64));

                loop {
                    timer.tick().await;

                    yield docker
                        .top_processes(
                            id,
                            Some(bollard::query_parameters::TopOptions {
                                ps_args: "-o user,pid,ppid,pcpu,etime,tty,time,args".to_owned(),
                            }),
                        )
                        .await
                }
            }
            .map_err(anyhow::Error::from)
            .map_ok(Into::into)
            .boxed(),
            Self::Podman(container) => container
                .top_stream(
                    &podman_api::opts::ContainerTopOpts::builder()
                        .delay(delay)
                        .ps_args("user,pid,ppid,pcpu,etime,tty,time,args")
                        .build(),
                )
                .map_err(anyhow::Error::from)
                .map_ok(Into::into)
                .boxed(),
        }
    }

    pub(crate) async fn rename(&self, name: &str) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => docker
                .rename_container(
                    id,
                    bollard::query_parameters::RenameContainerOptions {
                        name: name.to_owned(),
                    },
                )
                .await
                .map_err(anyhow::Error::from),
            Self::Podman(container) => container.rename(name).await.map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn restart(&self, force: bool) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => docker
                .restart_container(
                    id,
                    Some(bollard::query_parameters::RestartContainerOptions {
                        t: force.then_some(0),
                        ..Default::default()
                    }),
                )
                .await
                .map_err(anyhow::Error::from),

            Self::Podman(container) => if force {
                container.restart().await
            } else {
                container.restart_with_timeout(0).await
            }
            .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn unpause(&self) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => docker
                .unpause_container(id)
                .await
                .map_err(anyhow::Error::from),
            Self::Podman(container) => container.unpause().await.map_err(anyhow::Error::from),
        }
    }
}
