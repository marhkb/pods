mod pod_creation_page;
mod pod_details_page;
mod pod_menu_button;
mod pod_row;
mod pods_panel;

pub(crate) use pod_creation_page::PodCreationPage;
pub(crate) use pod_details_page::PodDetailsPage;
pub(crate) use pod_menu_button::PodMenuButton;
pub(crate) use pod_row::PodRow;
pub(crate) use pods_panel::PodsPanel;

use crate::model;

fn pod_status_css_class(status: model::PodStatus) -> &'static str {
    use model::PodStatus::*;

    match status {
        Created => "pod-status-created",
        Dead => "pod-status-dead",
        Degraded => "pod-status-degraded",
        Exited => "pod-status-exited",
        Paused => "pod-status-paused",
        Restarting => "pod-status-restarting",
        Running => "pod-status-running",
        Stopped => "pod-status-stopped",
        Unknown => "pod-status-unknown",
    }
}
