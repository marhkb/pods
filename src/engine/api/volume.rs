use crate::engine;

#[derive(Debug)]
pub(crate) enum Volume {
    Docker {
        docker: bollard::Docker,
        name: String,
    },
    Podman(podman_api::api::Volume),
}

impl Volume {
    pub(crate) async fn inspect(&self) -> anyhow::Result<engine::dto::Volume> {
        match self {
            Self::Docker { docker, name: id } => docker
                .inspect_volume(id)
                .await
                .map_err(anyhow::Error::from)
                .map(Into::into),
            Self::Podman(volume) => volume
                .inspect()
                .await
                .map_err(anyhow::Error::from)
                .map(Into::into),
        }
    }

    pub(crate) async fn json(&self) -> anyhow::Result<String> {
        match self {
            Self::Docker { docker, name: id } => docker
                .inspect_volume(id)
                .await
                .map_err(anyhow::Error::from)
                .and_then(|response| {
                    serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                }),
            Self::Podman(volume) => volume
                .inspect()
                .await
                .map_err(anyhow::Error::from)
                .and_then(|response| {
                    serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                }),
        }
    }

    pub(crate) async fn remove(&self, force: bool) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, name } => docker
                .remove_volume(
                    name,
                    Some(bollard::query_parameters::RemoveVolumeOptions { force }),
                )
                .await
                .map_err(anyhow::Error::from),
            Self::Podman(volume) => if force {
                volume.remove().await
            } else {
                volume.delete().await
            }
            .map_err(anyhow::Error::from),
        }
    }
}
