pub(crate) enum Credentials {
    BasicAuth { username: String, password: String },
    IdentityToken(String),
}

impl From<Credentials> for bollard::auth::DockerCredentials {
    fn from(value: Credentials) -> Self {
        match value {
            Credentials::BasicAuth { username, password } => Self {
                username: Some(username),
                password: Some(password),
                ..Default::default()
            },
            Credentials::IdentityToken(token) => Self {
                identitytoken: Some(token),
                ..Default::default()
            },
        }
    }
}

impl From<Credentials> for podman_api::opts::RegistryAuth {
    fn from(value: Credentials) -> Self {
        match value {
            Credentials::BasicAuth { username, password } => Self::builder()
                .username(username)
                .password(password)
                .build(),
            Credentials::IdentityToken(token) => Self::token(token),
        }
    }
}
