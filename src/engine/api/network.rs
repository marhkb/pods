use crate::engine;

#[derive(Debug)]
pub(crate) enum Network {
    Docker { docker: bollard::Docker, id: String },
    Podman(podman_api::api::Network),
}

impl Network {
    pub(crate) async fn remove(&self, force: bool) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => {
                docker.remove_network(id).await.map_err(anyhow::Error::from)
            }
            Self::Podman(network) => if force {
                network.remove().await
            } else {
                network.delete().await
            }
            .map(|_| ())
            .map_err(anyhow::Error::from),
        }
    }
}
