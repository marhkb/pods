#[derive(Default)]
pub(crate) struct ImagePullOpts {
    pub(crate) policy: ImagePullPolicy,
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
            .policy(value.policy.into())
            .reference(value.reference)
            .quiet(false)
            .build()
    }
}

// Podman-Only
#[derive(Clone, Copy, Default)]
pub(crate) enum ImagePullPolicy {
    #[default]
    Always,
    Missing,
}

impl From<ImagePullPolicy> for podman_api::opts::PullPolicy {
    fn from(value: ImagePullPolicy) -> Self {
        match value {
            ImagePullPolicy::Always => Self::Always,
            ImagePullPolicy::Missing => Self::Missing,
        }
    }
}
