mod container_creation_page;
mod container_details_panel;
mod container_logs_panel;
mod container_menu_button;
mod container_page;
mod container_rename_dialog;
mod container_row;
mod containers_group;
mod containers_panel;
mod env_var_row;
mod port_mapping_row;
mod volume_row;

pub(crate) use self::container_creation_page::ContainerCreationPage;
pub(crate) use self::container_details_panel::ContainerDetailsPanel;
pub(crate) use self::container_logs_panel::ContainerLogsPanel;
pub(crate) use self::container_menu_button::ContainerMenuButton;
pub(crate) use self::container_page::ContainerPage;
pub(crate) use self::container_rename_dialog::ContainerRenameDialog;
pub(crate) use self::container_row::ContainerRow;
pub(crate) use self::containers_group::ContainersGroup;
pub(crate) use self::containers_panel::ContainersPanel;
pub(crate) use self::env_var_row::EnvVarRow;
pub(crate) use self::port_mapping_row::PortMappingRow;
pub(crate) use self::volume_row::VolumeRow;
use crate::model;

fn container_status_css_class(status: model::ContainerStatus) -> &'static str {
    use model::ContainerStatus::*;

    match status {
        Configured => "container-status-configured",
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
