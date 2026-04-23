#[derive(Clone, Default)]
pub(crate) struct VolumeCreateOpts {
    pub(crate) name: Option<String>,
}

impl From<VolumeCreateOpts> for bollard::plugin::VolumeCreateRequest {
    fn from(value: VolumeCreateOpts) -> Self {
        bollard::config::VolumeCreateRequest {
            name: value.name,
            ..Default::default()
        }
    }
}

impl From<VolumeCreateOpts> for podman_api::opts::VolumeCreateOpts {
    fn from(value: VolumeCreateOpts) -> Self {
        podman_api::opts::VolumeCreateOpts::builder()
            .name(value.name)
            .build()
    }
}
