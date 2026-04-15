use std::collections::HashSet;

use gtk::glib;

use crate::engine;

pub(crate) enum Image {
    Summary(engine::dto::ImageSummary),
    Inspection(engine::dto::ImageInspection),
}

impl Image {
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Summary(dto) => &dto.id,
            Self::Inspection(dto) => &dto.summary.id,
        }
    }
}

pub(crate) struct ImageSummary {
    pub(crate) created: i64,
    pub(crate) dangling: bool,
    pub(crate) id: String,
    pub(crate) repo_tags: HashSet<String>,
    pub(crate) size: u64,
}

impl From<bollard::plugin::ImageSummary> for ImageSummary {
    fn from(value: bollard::plugin::ImageSummary) -> Self {
        Self {
            created: value.created,
            dangling: value.repo_tags.is_empty(),
            id: value.id,
            repo_tags: HashSet::from_iter(value.repo_tags),
            size: value.size as u64,
        }
    }
}

impl From<podman_api::models::LibpodImageSummary> for ImageSummary {
    fn from(value: podman_api::models::LibpodImageSummary) -> Self {
        Self {
            created: value.created.unwrap_or(0),
            dangling: value.dangling.unwrap_or(false),
            id: value.id.unwrap_or_default(),
            repo_tags: HashSet::from_iter(value.repo_tags.unwrap_or_default()),
            size: value.size.unwrap_or(0) as u64,
        }
    }
}

pub(crate) struct ImageDetails {
    pub(crate) architecture: Option<String>,
    pub(crate) author: Option<String>,
    pub(crate) cmd: Option<String>,
    pub(crate) comment: Option<String>,
    pub(crate) entrypoint: Option<String>,
    pub(crate) exposed_ports: Vec<String>,
    pub(crate) shared_size: Option<u64>,
    pub(crate) virtual_size: Option<u64>,
}

pub(crate) struct ImageInspection {
    pub(crate) summary: engine::dto::ImageSummary,
    pub(crate) details: engine::dto::ImageDetails,
}

impl From<bollard::plugin::ImageInspect> for ImageInspection {
    fn from(value: bollard::plugin::ImageInspect) -> Self {
        let (cmd, entry_point, exposed_ports) = value
            .config
            .map(|config| (config.cmd, config.entrypoint, config.exposed_ports))
            .unwrap_or_default();

        Self {
            summary: engine::dto::ImageSummary {
                created: value
                    .created
                    .and_then(|created| glib::DateTime::from_iso8601(&created, None).ok())
                    .map(|created| created.to_unix())
                    .unwrap_or(0),
                dangling: value
                    .repo_tags
                    .as_ref()
                    .map(|repo_tags| repo_tags.is_empty())
                    .unwrap_or(true),
                id: value.id.unwrap_or_default(),
                repo_tags: HashSet::from_iter(value.repo_tags.unwrap_or_default()),
                size: value
                    .size
                    .filter(|size| *size >= 0)
                    .map(|size| size as u64)
                    .unwrap_or_default(),
            },

            details: engine::dto::ImageDetails {
                architecture: value.architecture,
                author: value.author,
                cmd: cmd.and_then(|cmd| {
                    if cmd.is_empty() {
                        None
                    } else {
                        Some(cmd.join(" "))
                    }
                }),
                comment: value.comment,
                entrypoint: entry_point.and_then(|entry_point| {
                    if entry_point.is_empty() {
                        None
                    } else {
                        Some(entry_point.join(" "))
                    }
                }),
                exposed_ports: exposed_ports.unwrap_or_default(),
                shared_size: None,
                virtual_size: None,
            },
        }
    }
}

impl From<podman_api::models::InspectImageResponseLibpod> for ImageInspection {
    fn from(value: podman_api::models::InspectImageResponseLibpod) -> Self {
        let (cmd, entry_point, exposed_ports) = value
            .config
            .map(|config| (config.cmd, config.entrypoint, config.exposed_ports))
            .unwrap_or_default();

        Self {
            summary: ImageSummary {
                created: value
                    .created
                    .map(|date_time| date_time.timestamp())
                    .unwrap_or(0),
                dangling: value
                    .repo_tags
                    .as_ref()
                    .map(|repo_tags| repo_tags.is_empty())
                    .unwrap_or(true),
                id: value.id.unwrap_or_default(),
                repo_tags: HashSet::from_iter(value.repo_tags.unwrap_or_default()),
                size: value
                    .size
                    .filter(|size| *size >= 0)
                    .map(|size| size as u64)
                    .unwrap_or_default(),
            },
            details: ImageDetails {
                architecture: value.architecture,
                author: value.author,
                cmd: cmd.and_then(|cmd| {
                    if cmd.is_empty() {
                        None
                    } else {
                        Some(cmd.join(" "))
                    }
                }),
                comment: value.comment,
                entrypoint: entry_point.and_then(|entry_point| {
                    if entry_point.is_empty() {
                        None
                    } else {
                        Some(entry_point.join(" "))
                    }
                }),
                exposed_ports: exposed_ports
                    .map(|exposed_ports| exposed_ports.into_keys().collect())
                    .unwrap_or_default(),
                shared_size: None,
                virtual_size: value
                    .virtual_size
                    .filter(|virtual_size| *virtual_size >= 0)
                    .map(|virtual_| virtual_ as u64),
            },
        }
    }
}
