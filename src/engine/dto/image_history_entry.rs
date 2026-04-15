pub(crate) struct ImageHistoryEntry {
    pub(crate) comment: Option<String>,
    pub(crate) created: Option<i64>,
    pub(crate) created_by: Option<String>,
    pub(crate) id: Option<String>,
    pub(crate) size: Option<u64>,
    pub(crate) tags: Vec<String>,
}

impl From<bollard::plugin::HistoryResponseItem> for ImageHistoryEntry {
    fn from(value: bollard::plugin::HistoryResponseItem) -> Self {
        Self {
            comment: Some(value.comment).filter(|comment| !comment.is_empty()),
            created: (value.created >= 0).then_some(value.created),
            created_by: Some(value.created_by).filter(|created_by| !created_by.is_empty()),
            id: Some(value.id).filter(|id| !id.is_empty()),
            size: (value.size >= 0).then_some(value.size as u64),
            tags: value.tags,
        }
    }
}

impl From<podman_api::models::HistoryResponse> for ImageHistoryEntry {
    fn from(value: podman_api::models::HistoryResponse) -> Self {
        Self {
            comment: value.comment,
            created: value.created,
            created_by: value.created_by,
            id: value.id,
            size: value.size.map(|size| size as u64),
            tags: value.tags.unwrap_or_default(),
        }
    }
}
