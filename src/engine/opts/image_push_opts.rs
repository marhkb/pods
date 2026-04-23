use crate::engine;

#[derive(Clone, Default)]
pub(crate) struct ImagePushOpts {
    pub(crate) credentials: Option<engine::auth::Credentials>,
    pub(crate) repo: String,
    pub(crate) tag: String,
    pub(crate) tls_verify: bool,
}

impl From<ImagePushOpts> for bollard::query_parameters::PushImageOptions {
    fn from(value: ImagePushOpts) -> Self {
        Self {
            tag: Some(value.tag),
            ..Default::default()
        }
    }
}

impl From<ImagePushOpts> for podman_api::opts::ImagePushOpts {
    fn from(mut value: ImagePushOpts) -> Self {
        let mut builder = podman_api::opts::ImagePushOpts::builder()
            .destination(format!("{}:{}", value.repo, value.tag))
            .quiet(false)
            .tls_verify(value.tls_verify);

        if let Some(credentials) = value.credentials.take() {
            builder = builder.auth(credentials.into());
        }

        builder.build()
    }
}
