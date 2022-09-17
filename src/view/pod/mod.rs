mod creation_page;
mod details_page;
mod menu_button;
mod row;

pub(crate) use creation_page::CreationPage;
pub(crate) use details_page::DetailsPage;
pub(crate) use menu_button::MenuButton;
pub(crate) use row::Row;

use crate::model;

fn pod_status_css_class(status: model::PodStatus) -> &'static str {
    use model::PodStatus::*;

    match status {
        Created => "pod-status-created",
        Dead => "pod-status-dead",
        Degraded => "pod-status-degraded",
        Error => "pod-status-error",
        Exited => "pod-status-exited",
        Paused => "pod-status-paused",
        Restarting => "pod-status-restarting",
        Running => "pod-status-running",
        Stopped => "pod-status-stopped",
        Unknown => "pod-status-unknown",
    }
}
