#[derive(serde::Deserialize)]
pub(crate) struct ImagePushReport {
    pub(crate) stream: Option<String>,
    pub(crate) error: Option<String>,
}

impl From<bollard::plugin::PushImageInfo> for ImagePushReport {
    fn from(value: bollard::plugin::PushImageInfo) -> Self {
        Self {
            stream: value.status,
            error: value.error_detail.map(|detail| {
                detail
                    .message
                    .unwrap_or_else(|| detail.code.map(|code| code.to_string()).unwrap_or_default())
            }),
        }
    }
}

pub(crate) struct PodmanImagePushReport(pub(crate) String);

impl TryFrom<PodmanImagePushReport> for ImagePushReport {
    type Error = anyhow::Error;

    fn try_from(value: PodmanImagePushReport) -> Result<Self, Self::Error> {
        serde_json::from_str(&value.0).map_err(anyhow::Error::from)
    }
}
