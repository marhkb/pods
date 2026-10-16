use adw::prelude::*;
use glib::clone;
use gtk::glib;
use gtk::glib::clone::Downgrade;

use crate::expression::ExpressionExt;
use crate::gettext::gettext;
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

pub(crate) fn rename<W>(widget: &W, container: Option<&model::Container>)
where
    W: IsA<gtk::Widget> + Downgrade<Weak = glib::WeakRef<W>>,
{
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
                    &gettext("Could not rename container"),
                    &gettext("Container has been deleted"),
                );
            }
        ));

        container
            .property_expression_weak("name")
            .map(|name: String| {
                format!(
                    "{}\n<span weight=\"bold\">“{}”</span>",
                    gettext("Rename Container"),
                    name
                )
            })
            .bind(&dialog, "heading", Some(container));

        dialog.add_responses(&[
            ("cancel", &gettext("_Cancel")),
            ("rename", &gettext("_Rename")),
        ]);
        dialog.set_default_response(Some("rename"));
        dialog.set_close_response("cancel");
        dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);

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
                                    &gettext("Could not rename container"),
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

pub(crate) fn safe_remove<W>(widget: &W, container: Option<model::Container>)
where
    W: IsA<gtk::Widget> + Downgrade<Weak = glib::WeakRef<W>>,
{
    let dialog = adw::AlertDialog::builder()
        .heading(gettext("Delete Container?"))
        .body_use_markup(true)
        .body(gettext(
            "All settings and all changes made within the container will be irreversibly lost",
        ))
        .build();

    dialog.add_responses(&[
        ("cancel", &gettext("_Cancel")),
        ("delete", &gettext("_Delete")),
    ]);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

    if glib::MainContext::default().block_on(dialog.choose_future(Some(widget))) == "delete" {
        remove(widget, container)
    }
}

macro_rules! container_action {
    (fn $name:ident => $action:ident($($param:literal),*) => $error:tt) => {
        pub(crate) fn $name<W>(widget: &W, container: Option<crate::model::Container>)
        where
            W: gtk::glib::prelude::IsA<gtk::Widget> + gtk::glib::clone::Downgrade<Weak = gtk::glib::WeakRef<W>>,
        {
            let Some(container) = container else { return; };

            container.$action(
                $($param,)*
                gtk::glib::clone!(#[weak] widget, move |result| if let Err(e) = result {
                    crate::utils::show_error_toast(
                        &widget,
                        &$error,
                        &e.to_string()
                    );
                }),
            );
        }
    };
}

container_action!(fn start => start() => { gettextrs::gettext("Could not start container") });
container_action!(fn stop => stop(false) => { gettextrs::gettext("Could not stop container") });
container_action!(fn kill => stop(true) => { gettextrs::gettext("Could not kill container") });
container_action!(fn restart => restart(false) => { gettextrs::gettext("Could not restart container") });
container_action!(fn pause => pause() => { gettextrs::gettext("Could not pause container") });
container_action!(fn resume => resume() => { gettextrs::gettext("Could not resume container") });
container_action!(fn remove => remove() => { gettextrs::gettext("Could not delete container") });
