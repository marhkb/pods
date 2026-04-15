use crate::engine;

pub(crate) struct ImagePushOpts {
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

impl ImagePushOpts {
    pub(crate) fn into_podman(
        self,
        repo: String,
        credentials: Option<engine::auth::Credentials>,
    ) -> podman_api::opts::ImagePushOpts {
        let mut builder = podman_api::opts::ImagePushOpts::builder()
            .destination(format!("{repo}:{}", self.tag))
            .quiet(false)
            .tls_verify(self.tls_verify);

        if let Some(credentials) = credentials {
            builder = builder.auth(credentials.into());
        }

        builder.build()
    }
}
