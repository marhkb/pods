use crate::engine;

pub(crate) enum Networks {
    Docker(bollard::Docker),
    Podman(podman_api::api::Networks),
}

impl Networks {
    pub(crate) fn get(&self, id: impl Into<String>) -> engine::api::Network {
        match self {
            Self::Docker(docker) => engine::api::Network::Docker {
                docker: docker.clone(),
                id: id.into(),
            },
            Self::Podman(networks) => engine::api::Network::Podman(networks.get(id.into())),
        }
    }
}

impl Networks {
    pub(crate) async fn list(&self) -> anyhow::Result<Vec<engine::dto::Network>> {
        match self {
            Self::Docker(docker) => docker
                .list_networks(Option::<bollard::query_parameters::ListNetworksOptions>::None)
                .await
                .map_err(anyhow::Error::from)
                .map(|networks| networks.into_iter().map(Into::into).collect()),
            Self::Podman(networks) => networks
                .list(&Default::default())
                .await
                .map_err(anyhow::Error::from)
                .map(|networks| networks.into_iter().map(Into::into).collect()),
        }
    }
}
