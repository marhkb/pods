use std::collections::HashMap;

pub(crate) struct ImageBuildOpts {
    pub(crate) dockerfile: String,
    pub(crate) labels: HashMap<String, String>,
    pub(crate) tag: Option<String>,
}

impl From<ImageBuildOpts> for bollard::query_parameters::BuildImageOptions {
    fn from(value: ImageBuildOpts) -> Self {
        Self {
            dockerfile: value.dockerfile,
            t: value.tag,
            labels: Some(value.labels),
            ..Default::default()
        }
    }
}

impl ImageBuildOpts {
    pub(crate) fn into_podman(
        self,
        context_dir: impl Into<String>,
    ) -> podman_api::opts::ImageBuildOpts {
        let mut builder = podman_api::opts::ImageBuildOpts::builder(context_dir)
            .dockerfile(self.dockerfile)
            .labels(self.labels);

        if let Some(tag) = self.tag {
            builder = builder.tag(tag)
        }

        builder.build()
    }
}
