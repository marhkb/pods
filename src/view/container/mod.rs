mod container_creation_page;
mod container_details_page;
mod container_log_page;
mod container_menu_button;
mod container_properties_group;
mod container_rename_dialog;
mod container_resources_quick_reference_group;
mod container_row;
mod containers_count_bar;
mod containers_group;
mod containers_panel;
mod env_var_row;
mod port_mapping_row;
mod volume_row;

pub(crate) use self::container_creation_page::ContainerCreationPage;
pub(crate) use self::container_details_page::ContainerDetailsPage;
pub(crate) use self::container_log_page::ContainerLogPage;
pub(crate) use self::container_menu_button::ContainerMenuButton;
pub(crate) use self::container_properties_group::ContainerPropertiesGroup;
pub(crate) use self::container_rename_dialog::ContainerRenameDialog;
pub(crate) use self::container_resources_quick_reference_group::ContainerResourcesQuickReferenceGroup;
pub(crate) use self::container_row::ContainerRow;
pub(crate) use self::containers_count_bar::ContainersCountBar;
pub(crate) use self::containers_group::ContainersGroup;
pub(crate) use self::containers_panel::ContainersPanel;
pub(crate) use self::env_var_row::EnvVarRow;
pub(crate) use self::port_mapping_row::PortMappingRow;
pub(crate) use self::volume_row::VolumeRow;
use crate::model;

fn container_status_css_class(status: model::ContainerStatus) -> &'static str {
    use model::ContainerStatus::*;

    match status {
        Created => "container-status-created",
        Dead => "container-status-dead",
        Exited => "container-status-exited",
        Paused => "container-status-paused",
        Restarting => "container-status-restarting",
        Running => "container-status-running",
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
