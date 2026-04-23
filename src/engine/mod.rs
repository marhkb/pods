pub(crate) mod api;
pub(crate) mod auth;
pub(crate) mod conn;
pub(crate) mod dto;
pub(crate) mod opts;

use futures::StreamExt;
use futures::TryStreamExt;
use futures::stream::BoxStream;

use crate::engine;

#[derive(Clone, Debug, serde::Serialize)]
#[serde(untagged)]
pub(crate) enum Response<D, P> {
    Docker(D),
    Podman(P),
}

#[derive(Clone, Debug)]
pub(crate) struct Capabilities {
    pub(crate) kube_generation: bool,
    pub(crate) manual_health_check: bool,
    pub(crate) pods: bool,
    pub(crate) privileged_containers: bool,
    pub(crate) prune_external_images: bool,
    pub(crate) push_image_with_tls_verify: bool,
    pub(crate) prune_all_volumes: bool,
    pub(crate) prune_volumes_until: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum Engine {
    Docker(bollard::Docker),
    Podman(podman_api::Podman),
}

impl Engine {
    pub(crate) async fn new<U: AsRef<str>>(uri: U) -> anyhow::Result<Self> {
        let docker = bollard::Docker::connect_with_host(uri.as_ref())?;

        let components = docker.version().await?.components.unwrap_or_default();

        if components
            .iter()
            .any(|component| &component.name == "Engine")
        {
            Ok(Self::Docker(docker))
        } else if components
            .iter()
            .any(|component| &component.name == "Podman Engine")
        {
            Ok(Self::Podman(podman_api::Podman::new(uri)?))
        } else {
            Err(anyhow::anyhow!("no suitable engine detected"))
        }
    }

    pub(crate) const fn capabilities(&self) -> Capabilities {
        match self {
            Self::Docker(_) => Capabilities {
                kube_generation: false,
                manual_health_check: false,
                pods: false,
                privileged_containers: false,
                prune_external_images: false,
                push_image_with_tls_verify: false,
                prune_all_volumes: true,
                prune_volumes_until: false,
            },
            Self::Podman(_) => Capabilities {
                kube_generation: true,
                manual_health_check: true,
                pods: true,
                privileged_containers: true,
                prune_external_images: true,
                push_image_with_tls_verify: true,
                prune_all_volumes: false,
                prune_volumes_until: true,
            },
        }
    }
}

impl Engine {
    pub(crate) fn containers(&self) -> engine::api::Containers {
        match self {
            Self::Docker(docker) => engine::api::Containers::Docker(docker.to_owned()),
            Self::Podman(podman) => engine::api::Containers::Podman(podman.containers()),
        }
    }

    pub(crate) fn images(&self) -> engine::api::Images {
        match self {
            Self::Docker(docker) => engine::api::Images::Docker(docker.to_owned()),
            Self::Podman(podman) => engine::api::Images::Podman(podman.images()),
        }
    }

    pub(crate) fn pods(&self) -> engine::api::Pods {
        match self {
            Self::Docker(_) => engine::api::Pods::Docker,
            Self::Podman(podman) => engine::api::Pods::Podman(podman.pods()),
        }
    }

    pub(crate) fn volumes(&self) -> engine::api::Volumes {
        match self {
            Self::Docker(docker) => engine::api::Volumes::Docker(docker.to_owned()),
            Self::Podman(podman) => engine::api::Volumes::Podman(podman.volumes()),
        }
    }
}

impl Engine {
    pub(crate) async fn info(&self) -> anyhow::Result<engine::dto::Info> {
        match self {
            Self::Docker(docker) => docker
                .info()
                .await
                .map_err(anyhow::Error::from)
                .map(Into::into),
            Self::Podman(podman) => podman
                .info()
                .await
                .map_err(anyhow::Error::from)
                .map(Into::into),
        }
    }

    pub(crate) async fn json(&self) -> anyhow::Result<String> {
        match self {
            Self::Docker(docker) => {
                docker
                    .info()
                    .await
                    .map_err(anyhow::Error::from)
                    .and_then(|response| {
                        serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                    })
            }
            Self::Podman(podman) => {
                podman
                    .info()
                    .await
                    .map_err(anyhow::Error::from)
                    .and_then(|response| {
                        serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                    })
            }
        }
    }

    pub(crate) async fn ping(&self) -> anyhow::Result<()> {
        match self {
            Self::Docker(docker) => docker.ping().await.map_err(anyhow::Error::from).map(|_| ()),
            Self::Podman(podman) => podman.ping().await.map_err(anyhow::Error::from).map(|_| ()),
        }
    }

    pub(crate) fn events(&self) -> BoxStream<'_, anyhow::Result<engine::dto::Event>> {
        match self {
            Self::Docker(docker) => docker
                .events(None)
                .map_err(anyhow::Error::from)
                .map_ok(engine::dto::Event::Docker)
                .boxed(),
            Self::Podman(podman) => podman
                .events(&Default::default())
                .map_err(anyhow::Error::from)
                .map_ok(engine::dto::Event::Podman)
                .boxed(),
        }
    }
}
