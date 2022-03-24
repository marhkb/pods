mod check_service_page;
mod connection_lost_page;
mod container_details_panel;
mod container_logs_panel;
mod container_page;
mod container_rename_dialog;
mod container_row;
mod container_row_simple;
mod containers_panel;
mod image_details_page;
mod image_row;
mod image_row_simple;
mod image_used_by_row;
mod images_panel;
mod images_prune_dialog;
mod leaflet_overlay;
mod property_row;
mod start_service_page;

pub(crate) use self::check_service_page::CheckServicePage;
pub(crate) use self::connection_lost_page::ConnectionLostPage;
pub(crate) use self::container_details_panel::ContainerDetailsPanel;
pub(crate) use self::container_page::ContainerPage;
pub(crate) use self::container_rename_dialog::ContainerRenameDialog;
pub(crate) use self::container_row::ContainerRow;
pub(crate) use self::container_row_simple::ContainerRowSimple;
pub(crate) use self::containers_panel::ContainersPanel;
pub(crate) use self::image_details_page::ImageDetailsPage;
pub(crate) use self::image_row::ImageRow;
pub(crate) use self::image_row_simple::ImageRowSimple;
pub(crate) use self::image_used_by_row::ImageUsedByRow;
pub(crate) use self::images_panel::ImagesPanel;
pub(crate) use self::images_prune_dialog::ImagesPruneDialog;
pub(crate) use self::leaflet_overlay::LeafletOverlay;
pub(crate) use self::property_row::PropertyRow;
pub(crate) use self::start_service_page::StartServicePage;
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
