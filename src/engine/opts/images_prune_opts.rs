use std::collections::HashMap;

pub(crate) struct ImagesPruneOpts {
    pub(crate) all: bool,
    pub(crate) external: bool,
    pub(crate) until: Option<i64>,
}

impl From<ImagesPruneOpts> for bollard::query_parameters::PruneImagesOptions {
    fn from(value: ImagesPruneOpts) -> Self {
        let mut filters = HashMap::with_capacity(2);
        if value.all {
            filters.insert("dangling".to_owned(), vec!["0".to_owned()]);
        }
        if let Some(until) = value.until {
            filters.insert("until".to_owned(), vec![until.to_string()]);
        }

        Self {
            filters: Some(filters),
        }
    }
}

impl From<ImagesPruneOpts> for podman_api::opts::ImagePruneOpts {
    fn from(value: ImagesPruneOpts) -> Self {
        Self::builder()
            .all(value.all)
            .external(value.external)
            .filter(
                value
                    .until
                    .map(|until| until.to_string())
                    .map(podman_api::opts::ImagePruneFilter::Until),
            )
            .build()
    }
}
