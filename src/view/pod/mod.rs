mod creation_page;
mod details_page;
mod menu_button;
mod row;

use adw::prelude::MessageDialogExtManual;
use adw::traits::MessageDialogExt;
pub(crate) use creation_page::CreationPage;
pub(crate) use details_page::DetailsPage;
use gettextrs::gettext;
use glib::clone;
use gtk::glib;
pub(crate) use menu_button::MenuButton;
pub(crate) use row::Row;

use crate::model;
use crate::utils;

pub(crate) fn pod_status_css_class(status: model::PodStatus) -> &'static str {
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

macro_rules! pod_action {
    (fn $name:ident => $action:ident($($param:literal),*) => $error:tt) => {
        fn $name(widget: &gtk::Widget) {
            use gtk::glib;

            if let Some(pod) = <gtk::Widget as gtk::prelude::ObjectExt>::property::<Option<crate::model::Pod>>(widget, "pod") {
                pod.$action(
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

pod_action!(fn start => start() => "Error on starting pod");
pod_action!(fn stop => stop(false) => "Error on stopping pod");
pod_action!(fn kill => stop(true) => "Error on killing pod");
pod_action!(fn restart => restart(false) => "Error on restarting pod");
pod_action!(fn pause => pause() => "Error on pausing pod");
pod_action!(fn resume => resume() => "Error on resuming pod");
pod_action!(fn delete => delete(false) => "Error on deleting pod");

fn show_delete_confirmation_dialog(widget: &gtk::Widget) {
    if let Some(pod) =
        <gtk::Widget as gtk::prelude::ObjectExt>::property::<Option<model::Pod>>(widget, "pod")
    {
        let first_container = pod.container_list().get(0);

        if pod.num_containers() > 0 || first_container.is_some() {
            let dialog = adw::MessageDialog::builder()
                .heading(&gettext("Confirm Forced Pod Deletion"))
                .body_use_markup(true)
                .body(
                    &match first_container.as_ref().map(|c| c.name()) {
                        Some(id) => gettext!(
                            // Translators: The "{}" is a placeholder for the pod name.
                            "Pod contains container <b>{}</b>. Deleting the pod will also delete all its containers.",
                            id
                        ),
                        None => gettext(
                           "Pod contains a container. Deleting the pod will also delete all its containers.",
                       ),
                    }

                )
                .modal(true)
                .transient_for(&utils::root(widget))
                .build();

            dialog.add_responses(&[
                ("cancel", &gettext("_Cancel")),
                ("delete", &gettext("_Force Delete")),
            ]);
            dialog.set_default_response(Some("cancel"));
            dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

            dialog.run_async(
                None,
                clone!(@weak widget, @weak pod => move |_, response| {
                    if response == "delete" {
                        delete(&widget);
                    }
                }),
            );
        } else {
            delete(widget);
        }
    }
}
