use gtk::glib;

#[derive(Debug)]
pub(crate) struct Volume {
    pub(crate) name: String,
    pub(crate) created_at: i64,
    pub(crate) driver: String,
    pub(crate) mountpoint: String,
}

impl From<bollard::plugin::Volume> for Volume {
    fn from(value: bollard::plugin::Volume) -> Self {
        Self {
            name: value.name,
            created_at: value
                .created_at
                .and_then(|created_at| glib::DateTime::from_iso8601(&created_at, None).ok())
                .map(|created_at| created_at.to_unix())
                .unwrap_or(0),
            driver: value.driver,
            mountpoint: value.mountpoint,
        }
    }
}

impl From<podman_api::models::Volume> for Volume {
    fn from(value: podman_api::models::Volume) -> Self {
        Self {
            name: value.name,
            created_at: value
                .created_at
                .and_then(|created_at| glib::DateTime::from_iso8601(&created_at, None).ok())
                .map(|created_at| created_at.to_unix())
                .unwrap_or(0),
            driver: value.driver,
            mountpoint: value.mountpoint,
        }
    }
}
