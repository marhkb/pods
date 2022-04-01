mod container_details_panel;
mod container_logs_panel;
mod container_page;
mod container_rename_dialog;
mod container_row;
mod container_row_simple;
mod containers_panel;

pub(crate) use self::container_details_panel::ContainerDetailsPanel;
pub(crate) use self::container_logs_panel::ContainerLogsPanel;
pub(crate) use self::container_page::ContainerPage;
pub(crate) use self::container_rename_dialog::ContainerRenameDialog;
pub(crate) use self::container_row::ContainerRow;
pub(crate) use self::container_row_simple::ContainerRowSimple;
pub(crate) use self::containers_panel::ContainersPanel;
use crate::model;

pub(crate) fn container_status_css_class(status: model::ContainerStatus) -> &'static str {
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
