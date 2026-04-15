use crate::engine;

pub(crate) enum Volumes {
    Docker(bollard::Docker),
    Podman(podman_api::api::Volumes),
}

impl Volumes {
    pub(crate) fn get(&self, name: impl Into<String>) -> engine::api::Volume {
        match self {
            Self::Docker(docker) => engine::api::Volume::Docker {
                docker: docker.clone(),
                name: name.into(),
            },
            Self::Podman(volumes) => engine::api::Volume::Podman(volumes.get(name.into())),
        }
    }
}

impl Volumes {
    pub(crate) async fn create(&self, name: String) -> anyhow::Result<String> {
        match self {
            Self::Docker(docker) => docker
                .create_volume(bollard::config::VolumeCreateRequest {
                    name: Some(name),
                    ..Default::default()
                })
                .await
                .map_err(anyhow::Error::from)
                .map(|volume| volume.name),
            Self::Podman(volumes) => volumes
                .create(
                    &podman_api::opts::VolumeCreateOpts::builder()
                        .name(name)
                        .build(),
                )
                .await
                .map_err(anyhow::Error::from)
                .map(|volume| volume.name.unwrap()),
        }
    }

    pub(crate) async fn list(&self) -> anyhow::Result<Vec<engine::dto::Volume>> {
        match self {
            Self::Docker(docker) => docker
                .list_volumes(Option::<bollard::query_parameters::ListVolumesOptions>::None)
                .await
                .map_err(anyhow::Error::from)
                .map(|response| {
                    response
                        .volumes
                        .unwrap_or_default()
                        .into_iter()
                        .map(Into::into)
                        .collect()
                }),
            Self::Podman(volumes) => volumes
                .list(&Default::default())
                .await
                .map_err(anyhow::Error::from)
                .map(|volumes| volumes.into_iter().map(Into::into).collect()),
        }
    }

    pub(crate) async fn prune(
        &self,
        opts: engine::opts::VolumesPruneOpts,
    ) -> anyhow::Result<String> {
        match self {
            Self::Docker(docker) => docker
                .prune_volumes(Some(opts))
                .await
                .map_err(anyhow::Error::from)
                .and_then(|response| {
                    serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                }),
            Self::Podman(volumes) => volumes
                .prune(&opts.into())
                .await
                .map_err(anyhow::Error::from)
                .and_then(|response| {
                    serde_json::to_string_pretty(&response).map_err(anyhow::Error::from)
                }),
        }
    }
}
