use std::collections::HashMap;

pub(crate) struct VolumesPruneOpts {
    // Docker only
    pub(crate) all: bool,
    // Podman only
    pub(crate) until: Option<i64>,
}

impl From<VolumesPruneOpts> for bollard::query_parameters::PruneVolumesOptions {
    fn from(value: VolumesPruneOpts) -> Self {
        Self {
            filters: Some(HashMap::from([(
                "all".to_owned(),
                vec![value.all.to_string()],
            )])),
        }
    }
}

impl From<VolumesPruneOpts> for podman_api::opts::VolumePruneOpts {
    fn from(value: VolumesPruneOpts) -> Self {
        Self::builder()
            .filter(
                value
                    .until
                    .map(|until| until.to_string())
                    .map(podman_api::opts::VolumePruneFilter::Until),
            )
            .build()
    }
}
