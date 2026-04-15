use futures::StreamExt;
use futures::TryStreamExt;
use futures::stream;
use futures::stream::BoxStream;

use crate::engine;

#[allow(clippy::large_enum_variant)]
pub(crate) enum Pod {
    Docker,
    Podman(podman_api::api::Pod),
}

impl Pod {
    pub(crate) async fn generate_kube_yaml(&self, service: bool) -> anyhow::Result<String> {
        match self {
            Self::Docker => {
                anyhow::bail!("kube generation is not supported by the Docker API")
            }
            Self::Podman(pod) => pod
                .generate_kube_yaml(service)
                .await
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn inspect(&self) -> anyhow::Result<engine::dto::PodInspection> {
        match self {
            Self::Docker => anyhow::bail!("pods are not supported by the Docker API"),
            Self::Podman(pod) => pod
                .inspect()
                .await
                .map_err(anyhow::Error::from)
                .map(Into::into),
        }
    }

    pub(crate) async fn json(&self) -> anyhow::Result<String> {
        match self {
            Self::Docker => anyhow::bail!("pods are not supported by the Docker API"),

            Self::Podman(pod) => {
                pod.inspect()
                    .await
                    .map_err(anyhow::Error::from)
                    .and_then(|response| {
                        serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                    })
            }
        }
    }

    pub(crate) async fn pause(&self) -> anyhow::Result<()> {
        match self {
            Self::Docker => anyhow::bail!("pods are not supported by the Docker API"),
            Self::Podman(pod) => pod.pause().await.map_err(anyhow::Error::from).map(|_| ()),
        }
    }

    pub(crate) async fn remove(&self, force: bool) -> anyhow::Result<()> {
        match self {
            Self::Docker => anyhow::bail!("pods are not supported by the Docker API"),
            Self::Podman(pod) => if force {
                pod.remove().await.map(|_| ())
            } else {
                pod.delete().await.map(|_| ())
            }
            .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn restart(&self, force: bool) -> anyhow::Result<()> {
        match self {
            Self::Docker => anyhow::bail!("pods are not supported by the Docker API"),
            Self::Podman(pod) => if force {
                pod.kill().await?;
                pod.start().await.map(|_| ())
            } else {
                pod.stop().await.map(|_| ())
            }
            .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn start(&self) -> anyhow::Result<()> {
        match self {
            Self::Docker => anyhow::bail!("pods are not supported by the Docker API"),
            Self::Podman(pod) => pod.start().await.map_err(anyhow::Error::from).map(|_| ()),
        }
    }

    pub(crate) async fn stop(&self, force: bool) -> anyhow::Result<()> {
        match self {
            Self::Docker => anyhow::bail!("pods are not supported by the Docker API"),
            Self::Podman(pod) => if force {
                pod.kill().await.map(|_| ())
            } else {
                pod.stop().await.map(|_| ())
            }
            .map_err(anyhow::Error::from),
        }
    }

    pub(crate) fn top_stream(
        &self,
        delay: usize,
    ) -> BoxStream<'_, anyhow::Result<engine::dto::Top>> {
        match self {
            Self::Docker => {
                stream::once(
                    async move { anyhow::bail!("pods are not supported by the Docker API") },
                )
                .boxed()
            }
            Self::Podman(pod) => pod
                .top_stream(&podman_api::opts::PodTopOpts::builder().delay(delay).build())
                .map_err(anyhow::Error::from)
                .map_ok(Into::into)
                .boxed(),
        }
    }

    pub(crate) async fn unpause(&self) -> anyhow::Result<()> {
        match self {
            Self::Docker => anyhow::bail!("pods are not supported by the Docker API"),
            Self::Podman(pod) => pod.unpause().await.map_err(anyhow::Error::from).map(|_| ()),
        }
    }
}
