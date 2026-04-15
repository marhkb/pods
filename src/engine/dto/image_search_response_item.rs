pub(crate) struct ImageSearchResponseItem {
    pub(crate) automated: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) index: Option<String>,
    pub(crate) is_official: bool,
    pub(crate) name: Option<String>,
    pub(crate) stars: u64,
    pub(crate) tag: Option<String>,
}

impl From<bollard::plugin::ImageSearchResponseItem> for ImageSearchResponseItem {
    fn from(value: bollard::plugin::ImageSearchResponseItem) -> Self {
        Self {
            automated: None,
            description: value.description,
            index: None,
            is_official: value.is_official.unwrap_or(false),
            name: value.name,
            stars: value.star_count.unwrap_or(0) as u64,
            tag: None,
        }
    }
}

impl From<podman_api::models::RegistrySearchResponse> for ImageSearchResponseItem {
    fn from(value: podman_api::models::RegistrySearchResponse) -> Self {
        Self {
            automated: value.automated,
            description: value.description,
            index: value.index,
            is_official: value
                .official
                .map(|official| !official.is_empty())
                .unwrap_or(false),
            name: value.name,
            stars: value.stars.unwrap_or(0) as u64,
            tag: value.tag,
        }
    }
}
