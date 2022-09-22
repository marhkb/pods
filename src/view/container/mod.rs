mod creation_page;
mod details_page;
mod health_check_page;
mod log_page;
mod menu_button;
mod properties_group;
mod rename_dialog;
mod resources_quick_reference_group;
mod row;

pub(crate) use creation_page::CreationPage;
pub(crate) use details_page::DetailsPage;
pub(crate) use health_check_page::HealthCheckPage;
pub(crate) use log_page::LogPage;
pub(crate) use menu_button::MenuButton;
pub(crate) use properties_group::PropertiesGroup;
pub(crate) use rename_dialog::RenameDialog;
pub(crate) use resources_quick_reference_group::ResourcesQuickReferenceGroup;
pub(crate) use row::Row;

use crate::model;

fn container_status_css_class(status: model::ContainerStatus) -> &'static str {
    use model::ContainerStatus::*;

    match status {
        Created => "container-status-created",
        Dead => "container-status-dead",
        Exited => "container-status-exited",
        Paused => "container-status-paused",
        Removing => "container-status-removing",
        Restarting => "container-status-restarting",
        Running => "container-status-running",
        Stopped => "container-status-stopped",
        Stopping => "container-status-stopping",
        Unknown => "container-status-unknown",
    }
}

fn container_health_status_css_class(status: model::ContainerHealthStatus) -> &'static str {
    use model::ContainerHealthStatus::*;

    match status {
        Starting => "container-health-status-checking",
        Healthy => "container-health-status-healthy",
        Unhealthy => "container-health-status-unhealthy",
        Unconfigured => "container-health-status-unconfigured",
        Unknown => "container-health-status-unknown",
    }
}
