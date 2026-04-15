use futures::StreamExt;
use futures::TryStreamExt;
use futures::future;
use futures::stream::BoxStream;

use crate::engine::{self};

pub(crate) enum Image {
    Docker { docker: bollard::Docker, id: String },
    Podman(podman_api::api::Image),
}

impl Image {
    pub(crate) async fn history(&self) -> anyhow::Result<Vec<engine::dto::ImageHistoryEntry>> {
        match self {
            Self::Docker { docker, id } => docker
                .image_history(id)
                .await
                .map_err(anyhow::Error::from)
                .map(|items| items.into_iter().map(Into::into).collect()),
            Self::Podman(image) => image
                .history()
                .await
                .map_err(anyhow::Error::from)
                .map(|items| items.into_iter().map(Into::into).collect()),
        }
    }

    pub(crate) async fn inspect(&self) -> anyhow::Result<engine::dto::ImageInspection> {
        match self {
            Self::Docker { docker, id } => docker
                .inspect_image(id)
                .await
                .map_err(anyhow::Error::from)
                .map(Into::into),
            Self::Podman(image) => image
                .inspect()
                .await
                .map_err(anyhow::Error::from)
                .map(Into::into),
        }
    }

    pub(crate) async fn json(&self) -> anyhow::Result<String> {
        match self {
            Self::Docker { docker, id } => docker
                .inspect_image(id)
                .await
                .map_err(anyhow::Error::from)
                .and_then(|response| {
                    serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                }),

            Self::Podman(image) => {
                image
                    .inspect()
                    .await
                    .map_err(anyhow::Error::from)
                    .and_then(|response| {
                        serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                    })
            }
        }
    }

    pub(crate) fn push(
        &self,
        repo: String,
        opts: engine::opts::ImagePushOpts,
        credentials: Option<engine::auth::Credentials>,
    ) -> BoxStream<'_, anyhow::Result<engine::dto::ImagePushReport>> {
        match self {
            Self::Docker { docker, .. } => docker
                .push_image(&repo, Some(opts), credentials.map(Into::into))
                .map_err(anyhow::Error::from)
                .map_ok(Into::into)
                .boxed(),
            Self::Podman(image) => image
                .push(&opts.into_podman(repo, credentials))
                .map_err(anyhow::Error::from)
                .map_ok(engine::dto::PodmanImagePushReport)
                .and_then(|report| future::ready(report.try_into()))
                .boxed(),
        }
    }

    pub(crate) async fn remove(&self, force: bool) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => docker
                .remove_image(
                    id,
                    Some(bollard::query_parameters::RemoveImageOptions {
                        force,
                        ..Default::default()
                    }),
                    None,
                )
                .await
                .map_err(anyhow::Error::from)
                .map(|_| ()),
            Self::Podman(image) => {
                if force {
                    image.remove().await.map_err(anyhow::Error::from)
                } else {
                    image.delete().await.map_err(anyhow::Error::from)
                }
            }
        }
    }

    pub(crate) async fn tag(&self, repo: String, tag: String) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => docker
                .tag_image(
                    id,
                    Some(bollard::query_parameters::TagImageOptions {
                        repo: Some(repo),
                        tag: Some(tag),
                    }),
                )
                .await
                .map_err(anyhow::Error::from),
            Self::Podman(image) => image
                .tag(
                    &podman_api::opts::ImageTagOpts::builder()
                        .repo(repo)
                        .tag(tag)
                        .build(),
                )
                .await
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn untag(&self, repo: &str, tag: &str) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, .. } => docker
                .remove_image(
                    &format!("{repo}:{tag}"),
                    Some(bollard::query_parameters::RemoveImageOptions {
                        noprune: true,
                        ..Default::default()
                    }),
                    None,
                )
                .await
                .map_err(anyhow::Error::from)
                .map(|_| ()),
            Self::Podman(image) => image
                .untag(
                    &podman_api::opts::ImageTagOpts::builder()
                        .repo(repo)
                        .tag(tag)
                        .build(),
                )
                .await
                .map_err(anyhow::Error::from)
                .map(|_| ()),
        }
    }
}
