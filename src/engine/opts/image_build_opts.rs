use std::collections::HashMap;

#[derive(Clone, Default)]
pub(crate) struct ImageBuildOpts {
    pub(crate) dockerfile: String,
    pub(crate) labels: HashMap<String, String>,
    pub(crate) path: String,
    pub(crate) tag: String,
}

impl From<ImageBuildOpts> for bollard::query_parameters::BuildImageOptions {
    fn from(value: ImageBuildOpts) -> Self {
        Self {
            dockerfile: value.dockerfile,
            t: Some(value.tag),
            labels: Some(value.labels),
            ..Default::default()
        }
    }
}

impl From<ImageBuildOpts> for podman_api::opts::ImageBuildOpts {
    fn from(value: ImageBuildOpts) -> Self {
        Self::builder(value.path)
            .dockerfile(value.dockerfile)
            .labels(value.labels)
            .tag(value.tag)
            .build()
    }
}
