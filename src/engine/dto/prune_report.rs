pub(crate) struct PruneReport {
    pub(crate) deleted: Vec<String>,
    pub(crate) space_reclaimed: u64,
}

impl From<bollard::plugin::ContainerPruneResponse> for PruneReport {
    fn from(value: bollard::plugin::ContainerPruneResponse) -> Self {
        Self {
            deleted: value.containers_deleted.unwrap_or_default(),
            space_reclaimed: value.space_reclaimed.unwrap_or(0) as u64,
        }
    }
}

impl From<bollard::plugin::ImagePruneResponse> for PruneReport {
    fn from(value: bollard::plugin::ImagePruneResponse) -> Self {
        Self {
            deleted: value
                .images_deleted
                .unwrap_or_default()
                .into_iter()
                .filter_map(|item| item.deleted)
                .collect(),
            space_reclaimed: value.space_reclaimed.unwrap_or(0) as u64,
        }
    }
}

impl From<bollard::plugin::VolumePruneResponse> for PruneReport {
    fn from(value: bollard::plugin::VolumePruneResponse) -> Self {
        Self {
            deleted: value.volumes_deleted.unwrap_or_default(),
            space_reclaimed: value.space_reclaimed.unwrap_or(0) as u64,
        }
    }
}

macro_rules! impl_podman_prune_report {
    ($podman_model:ty) => {
        impl From<Vec<$podman_model>> for PruneReport {
            fn from(value: Vec<$podman_model>) -> Self {
                let (deleted, space_reclaimed) = value.into_iter().fold(
                    (Vec::new(), 0),
                    |(mut deleted, space_reclaimed), report| {
                        if let Some(id) = report.id {
                            deleted.push(id);
                        }
                        (deleted, space_reclaimed + report.size.unwrap_or(0))
                    },
                );

                Self {
                    deleted,
                    space_reclaimed: space_reclaimed as u64,
                }
            }
        }
    };
}

impl_podman_prune_report!(podman_api::models::PruneReport);
impl_podman_prune_report!(podman_api::models::ContainersPruneReportLibpod);
