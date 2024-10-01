use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

pub(crate) fn container_status_css_class(status: model::ContainerStatus) -> &'static str {
    use model::ContainerStatus::*;

    match status {
        Running => "container-status-running",
        Unknown => "container-status-unknown",
        _ => "container-status-not-running",
    }
}

pub(crate) fn container_health_status_css_class(
    status: model::ContainerHealthStatus,
) -> &'static str {
    use model::ContainerHealthStatus::*;

    match status {
        Healthy => "container-health-status-healthy",
        Unhealthy => "container-health-status-unhealthy",
        Unknown => "container-health-status-unknown",
        _ => "container-health-status-not-running",
    }
}

pub(crate) fn container_status_combined_css_class(
    status: model::ContainerStatus,
    health_status: model::ContainerHealthStatus,
) -> &'static str {
    use model::ContainerStatus::*;

    match status {
        Running => {
            use model::ContainerHealthStatus::*;

            match health_status {
                Healthy => "container-health-status-healthy",
                Unhealthy => "container-health-status-unhealthy",
                _ => "container-status-running",
            }
        }
        Unknown => "container-status-unknown",
        _ => "container-status-not-running",
    }
}

pub(crate) fn rename(widget: &gtk::Widget, container: Option<&model::Container>) {
    if let Some(container) = container {
        let container_renamer = view::ContainerRenamer::from(container);

        let dialog: adw::AlertDialog = adw::AlertDialog::builder()
            .width_request(360)
            .heading_use_markup(true)
            .extra_child(&container_renamer)
            .build();

        container.connect_deleted(clone!(
            #[weak]
            widget,
            #[weak]
            dialog,
            move |_| {
                dialog.force_close();
                utils::show_error_toast(
                    &widget,
                    &gettext("Error renaming container"),
                    &gettext("Container has been deleted"),
                );
            }
        ));

        container
            .property_expression_weak("name")
            .chain_closure::<String>(closure!(|_: model::Container, name: String| {
                format!(
                    "{}\n<span weight=\"bold\">«{}»</span>",
                    gettext("Rename Container"),
                    name
                )
            }))
            .bind(&dialog, "heading", Some(container));

        dialog.add_responses(&[
            ("cancel", &gettext("_Cancel")),
            ("rename", &gettext("_Rename")),
        ]);
        dialog.set_default_response(Some("rename"));
        dialog.set_response_appearance("rename", adw::ResponseAppearance::Destructive);

        dialog.connect_response(
            Some("rename"),
            clone!(
                #[weak]
                widget,
                #[weak]
                container,
                #[weak]
                container_renamer,
                move |_, _| {
                    container.rename(
                        container_renamer.new_name(),
                        clone!(
                            #[weak]
                            widget,
                            move |result| if let Err(e) = result {
                                utils::show_error_toast(
                                    &widget,
                                    &gettext("Error renaming container"),
                                    &e.to_string(),
                                );
                            }
                        ),
                    );
                }
            ),
        );

        dialog.present(Some(widget));
    }
}

macro_rules! container_action {
    (fn $name:ident => $action:ident($($param:literal),*) => $error:tt) => {
        pub(crate) fn $name(widget: &gtk::Widget) {
            use gtk::glib;
            if let Some(container) = <gtk::Widget as gtk::prelude::ObjectExt>::property::<Option<crate::model::Container>>(widget, "container") {
                container.$action(
                    $($param,)*
                    glib::clone!(#[weak] widget, move |result| if let Err(e) = result {
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
