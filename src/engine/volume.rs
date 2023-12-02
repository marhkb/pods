use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct Volume {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    pub driver: String,
    pub labels: HashMap<String, String>,
    pub mountpoint: String,
    pub name: String,
    pub options: HashMap<String, String>,
    pub scope: String,
    // #[serde(rename = "Status")]
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub status: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_data: Option<VolumeUsageData>,
}

impl From<docker_api::models::Volume> for Volume {
    fn from(value: docker_api::models::Volume) -> Self {
        Self {
            name: value.name,
            created_at: value.created_at.map(|dt| format!("{}", dt.format("%+"))),
            driver: value.driver,
            mountpoint: value.mountpoint,
            labels: value.labels,
            options: value.options,
            scope: value.scope,
            usage_data: value.usage_data.map(Into::into),
        }
    }
}

impl From<podman_api::models::Volume> for Volume {
    fn from(value: podman_api::models::Volume) -> Self {
        Self {
            name: value.name,
            created_at: value.created_at,
            driver: value.driver,
            mountpoint: value.mountpoint,
            labels: value.labels,
            options: value.options,
            scope: value.scope,
            usage_data: value.usage_data.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct VolumeUsageData {
    #[serde(rename = "RefCount")]
    pub ref_count: i64,
    #[serde(rename = "Size")]
    pub size: i64,
}

impl From<docker_api::models::UsageData> for VolumeUsageData {
    fn from(value: docker_api::models::UsageData) -> Self {
        Self {
            ref_count: value.ref_count,
            size: value.size,
        }
    }
}

impl From<podman_api::models::VolumeUsageData> for VolumeUsageData {
    fn from(value: podman_api::models::VolumeUsageData) -> Self {
        Self {
            ref_count: value.ref_count,
            size: value.size,
        }
    }
}
