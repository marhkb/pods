use crate::model;

pub(crate) fn container_status_css_class(status: model::ContainerStatus) -> &'static str {
    use model::ContainerStatus::*;

    match status {
        Configured => "container-status-configured",
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

pub(crate) fn container_health_status_css_class(
    status: model::ContainerHealthStatus,
) -> &'static str {
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
        pub(crate) fn $name(widget: &gtk::Widget) {
            use gtk::glib;
            if let Some(container) = <gtk::Widget as gtk::prelude::ObjectExt>::property::<Option<crate::model::Container>>(widget, "container") {
                container.$action(
                    $($param,)*
                    glib::clone!(@weak widget => move |result| if let Err(e) = result {
                        crate::utils::show_error_toast(
                            &widget,
                            &$error,
                            &e.to_string()
                        );
                    }),
                );
            }
        }
    };
}

container_action!(fn start => start() => { gettextrs::gettext("Error on starting container") });
container_action!(fn stop => stop(false) => { gettextrs::gettext("Error on stopping container") });
container_action!(fn kill => stop(true) => { gettextrs::gettext("Error on killing container") });
container_action!(fn restart => restart(false) => { gettextrs::gettext("Error on restarting container") });
container_action!(fn pause => pause() => { gettextrs::gettext("Error on pausing container") });
container_action!(fn resume => resume() => { gettextrs::gettext("Error on resuming container") });
container_action!(fn delete => delete(false) => { gettextrs::gettext("Error on deleting container") });
