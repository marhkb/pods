mod container_inspection;
mod container_summary;
mod event;
mod image_summary;
mod info;
mod volume;

pub(crate) use container_inspection::ContainerInspection;
pub(crate) use container_summary::ContainerSummary;
pub(crate) use container_summary::PortMapping;
pub(crate) use event::Event;
use futures::StreamExt;
use futures::TryStreamExt;
pub(crate) use image_summary::ImageSummary;
pub(crate) use info::Info;
pub(crate) use volume::Volume;

use crate::engine;

#[derive(Clone, Debug)]
pub(crate) enum Engine {
    Docker(docker_api::Docker),
    Podman(podman_api::Podman),
}

impl Engine {
    pub(crate) async fn new<U: AsRef<str>>(uri: U) -> anyhow::Result<Self> {
        match podman_api::Podman::new(&uri) {
            Ok(podman) => match podman.ping().await {
                Ok(_) => Ok(Self::Podman(podman)),
                Err(_) => match docker_api::Docker::new(uri) {
                    Ok(docker) => docker
                        .ping()
                        .await
                        .map_err(Into::into)
                        .map(|_| Self::Docker(docker)),
                    Err(e) => Err(e.into()),
                },
            },
            Err(e) => Err(e.into()),
        }
    }

    pub(crate) async fn ping(&self) -> anyhow::Result<()> {
        match self {
            Self::Docker(docker) => docker.ping().await.map(|_| ()).map_err(anyhow::Error::from),
            Self::Podman(podman) => podman.ping().await.map(|_| ()).map_err(anyhow::Error::from),
        }
    }

    // TODO: version endpoint

    pub(crate) async fn info(&self) -> anyhow::Result<engine::Info> {
        match self {
            Self::Docker(docker) => docker
                .info()
                .await
                .map(Into::into)
                .map_err(anyhow::Error::from),
            Self::Podman(podman) => podman
                .info()
                .await
                .map(Into::into)
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn list_containers(
        &self,
        id: Option<String>,
    ) -> anyhow::Result<Vec<engine::ContainerSummary>> {
        match self {
            Self::Docker(docker) => docker
                .containers()
                .list(
                    &docker_api::opts::ContainerListOpts::builder()
                        .all(true)
                        .filter(id.map(docker_api::opts::ContainerFilter::Id))
                        .build(),
                )
                .await
                .map(|containers| containers.into_iter().map(Into::into).collect())
                .map_err(anyhow::Error::from),
            Self::Podman(podman) => podman
                .containers()
                .list(
                    &podman_api::opts::ContainerListOpts::builder()
                        .all(true)
                        .filter(
                            id.map(podman_api::Id::from)
                                .map(podman_api::opts::ContainerListFilter::Id),
                        )
                        .build(),
                )
                .await
                .map(|containers| containers.into_iter().map(Into::into).collect())
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn inspect_container(
        &self,
        id: String,
    ) -> anyhow::Result<engine::ContainerInspection> {
        match self {
            Self::Docker(docker) => docker
                .containers()
                .get(id)
                .inspect()
                .await
                .map(Into::into)
                .map_err(anyhow::Error::from),
            Self::Podman(podman) => podman
                .containers()
                .get(id)
                .inspect()
                .await
                .map(Into::into)
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn list_images(&self) -> anyhow::Result<Vec<engine::ImageSummary>> {
        match self {
            Self::Docker(docker) => docker
                .images()
                .list(&docker_api::opts::ImageListOpts::builder().all(true).build())
                .await
                .map(|images| images.into_iter().map(Into::into).collect())
                .map_err(anyhow::Error::from),
            Self::Podman(podman) => podman
                .images()
                .list(&podman_api::opts::ImageListOpts::builder().all(true).build())
                .await
                .map(|images| images.into_iter().map(Into::into).collect())
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn list_volumes(&self) -> anyhow::Result<Vec<engine::Volume>> {
        match self {
            Self::Docker(docker) => docker
                .volumes()
                .list(&docker_api::opts::VolumeListOpts::builder().build())
                .await
                .map(|volumes| {
                    volumes
                        .volumes
                        .unwrap()
                        .into_iter()
                        .map(Into::into)
                        .collect()
                })
                .map_err(anyhow::Error::from),
            Self::Podman(podman) => podman
                .volumes()
                .list(&podman_api::opts::VolumeListOpts::builder().build())
                .await
                .map(|volumes| volumes.into_iter().map(Into::into).collect())
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) fn events(
        &self,
    ) -> impl futures::Stream<Item = anyhow::Result<engine::Event>> + Unpin + '_ {
        match self {
            Self::Docker(docker) => docker
                .events(&docker_api::opts::EventsOpts::builder().build())
                .map_ok(Into::into)
                .map_err(anyhow::Error::from)
                .boxed(),
            Self::Podman(podman) => podman
                .events(&podman_api::opts::EventsOpts::builder().build())
                .map_ok(Into::into)
                .map_err(anyhow::Error::from)
                .boxed(),
        }
    }
}
