mod commit_page;
mod creation_page;
mod details_page;
mod files_get_page;
mod files_put_page;
mod health_check_page;
mod log_page;
mod menu_button;
mod properties_group;
mod rename_dialog;
mod resources_quick_reference_group;
mod row;
mod tty;
mod tty_page;

pub(crate) use self::commit_page::CommitPage;
pub(crate) use self::creation_page::CreationPage;
pub(crate) use self::details_page::DetailsPage;
pub(crate) use self::files_get_page::FilesGetPage;
pub(crate) use self::files_put_page::FilesPutPage;
pub(crate) use self::health_check_page::HealthCheckPage;
pub(crate) use self::log_page::LogPage;
pub(crate) use self::menu_button::MenuButton;
pub(crate) use self::properties_group::PropertiesGroup;
pub(crate) use self::rename_dialog::RenameDialog;
pub(crate) use self::resources_quick_reference_group::ResourcesQuickReferenceGroup;
pub(crate) use self::row::Row;
pub(crate) use self::tty::Tty;
pub(crate) use self::tty_page::TtyPage;
use crate::model;

fn container_status_css_class(status: model::ContainerStatus) -> &'static str {
    use model::ContainerStatus::*;

    match status {
        Created => "container-status-created",
        Dead => "container-status-dead",
        Exited => "container-status-exited",
        Initialized => "container-status-initialized",
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

macro_rules! container_action {
    (fn $name:ident => $action:ident($($param:literal),*) => $error:tt) => {
        fn $name(widget: &gtk::Widget) {
            use gtk::glib;
            if let Some(container) = <gtk::Widget as gtk::prelude::ObjectExt>::property::<Option<crate::model::Container>>(widget, "container") {
                container.$action(
                    $($param,)*
                    glib::clone!(@weak widget => move |result| if let Err(e) = result {
                        crate::utils::show_error_toast(
                            &widget,
                            &gettextrs::gettext($error),
                            &e.to_string()
                        );
                    }),
                );
            }
        }
    };
}

container_action!(fn start => start() => "Error on starting container");
container_action!(fn stop => stop(false) => "Error on stopping container");
container_action!(fn kill => stop(true) => "Error on killing container");
container_action!(fn restart => restart(false) => "Error on restarting container");
container_action!(fn pause => pause() => "Error on pausing container");
container_action!(fn resume => resume() => "Error on resuming container");
container_action!(fn delete => delete(false) => "Error on deleting container");
