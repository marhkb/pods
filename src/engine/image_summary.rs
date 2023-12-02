#[derive(Debug, Clone, PartialEq)]
pub struct ImageSummary {
    pub containers: Option<i64>,
    pub created: Option<i64>,
    pub dangling: Option<bool>,
    pub history: Option<Vec<String>>,
    pub id: Option<String>,
    pub repo_tags: Option<Vec<String>>,
    pub shared_size: Option<i64>,
    pub size: Option<i64>,
    pub virtual_size: Option<i64>,
}

impl From<docker_api::models::ImageSummary> for ImageSummary {
    fn from(value: docker_api::models::ImageSummary) -> Self {
        Self {
            containers: Some(value.containers as i64),
            created: Some(value.created as i64),
            dangling: None,
            history: None,
            id: Some(value.id),
            repo_tags: Some(value.repo_tags),
            shared_size: Some(value.shared_size),
            size: Some(value.virtual_size),
            virtual_size: Some(value.virtual_size),
        }
    }
}

impl From<podman_api::models::LibpodImageSummary> for ImageSummary {
    fn from(value: podman_api::models::LibpodImageSummary) -> Self {
        Self {
            containers: value.containers,
            created: value.created,
            dangling: value.dangling,
            history: value.history,
            id: value.id,
            repo_tags: value.repo_tags,
            shared_size: value.shared_size,
            size: value.virtual_size,
            virtual_size: value.virtual_size,
        }
    }
}
