use crate::engine;

#[allow(clippy::large_enum_variant)]
pub(crate) enum Pods {
    Docker,
    Podman(podman_api::api::Pods),
}

impl Pods {
    pub(crate) fn get(&self, id: impl Into<String>) -> engine::api::Pod {
        match self {
            Self::Docker => engine::api::Pod::Docker,
            Self::Podman(pods) => engine::api::Pod::Podman(pods.get(id.into())),
        }
    }
}

impl Pods {
    pub(crate) async fn create(&self, opts: engine::opts::PodCreateOpts) -> anyhow::Result<String> {
        match self {
            Self::Docker => anyhow::bail!("pods are not supported by the Docker API"),
            Self::Podman(pods) => pods
                .create(&opts.into())
                .await
                .map_err(anyhow::Error::from)
                .map(|pod| pod.id().to_string()),
        }
    }

    pub(crate) async fn list(&self) -> anyhow::Result<Vec<engine::dto::PodSummary>> {
        match self {
            Self::Docker => anyhow::bail!("pods are not supported by the Docker API"),
            Self::Podman(pods) => pods
                .list(&podman_api::opts::PodListOpts::builder().build())
                .await
                .map_err(anyhow::Error::from)
                .map(|summaries| summaries.into_iter().map(Into::into).collect()),
        }
    }

    pub(crate) async fn prune(&self) -> anyhow::Result<String> {
        match self {
            Self::Docker => anyhow::bail!("pods are not supported by the Docker API"),
            Self::Podman(pods) => {
                pods.prune()
                    .await
                    .map_err(anyhow::Error::from)
                    .and_then(|response| {
                        serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                    })
            }
        }
    }
}
