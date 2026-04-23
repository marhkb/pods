pub(crate) struct PodsPruneReport {
    pub(crate) deleted: Vec<String>,
}

impl From<Vec<podman_api::models::PodPruneReport>> for PodsPruneReport {
    fn from(value: Vec<podman_api::models::PodPruneReport>) -> Self {
        Self {
            deleted: value.into_iter().filter_map(|report| report.id).collect(),
        }
    }
}
