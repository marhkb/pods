use std::collections::HashMap;

#[derive(Clone, Default)]
pub(crate) struct ContainersPruneOpts {
    pub(crate) until: Option<i64>,
}

impl From<ContainersPruneOpts> for bollard::query_parameters::PruneContainersOptions {
    fn from(value: ContainersPruneOpts) -> Self {
        Self {
            filters: value
                .until
                .map(|until| HashMap::from([("until".to_owned(), vec![until.to_string()])])),
        }
    }
}

impl From<ContainersPruneOpts> for podman_api::opts::ContainerPruneOpts {
    fn from(value: ContainersPruneOpts) -> Self {
        Self::builder()
            .filter(
                value
                    .until
                    .map(|until| until.to_string())
                    .map(podman_api::opts::ContainerPruneFilter::Until),
            )
            .build()
    }
}
