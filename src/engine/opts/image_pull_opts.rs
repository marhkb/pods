#[derive(Clone, Default)]
pub(crate) struct ImagePullOpts {
    pub(crate) reference: String,
}

impl From<ImagePullOpts> for bollard::query_parameters::CreateImageOptions {
    fn from(value: ImagePullOpts) -> Self {
        Self {
            from_image: Some(value.reference),
            ..Default::default()
        }
    }
}

impl From<ImagePullOpts> for podman_api::opts::PullOpts {
    fn from(value: ImagePullOpts) -> Self {
        Self::builder()
            .reference(value.reference)
            .quiet(false)
            .build()
    }
}
