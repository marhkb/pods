mod container_details_panel;
mod container_logs_panel;
mod container_page;
mod container_rename_dialog;
mod container_row;
mod containers_group;
mod containers_panel;
mod env_var_row;
mod port_mapping_row;
mod volume_row;

use cascade::cascade;
use gettextrs::gettext;
use gtk::glib::clone;
use gtk::prelude::{Cast, DialogExtManual};
use gtk::traits::{DialogExt, GtkWindowExt, WidgetExt};
use gtk::{gio, glib};

pub(crate) use self::container_details_panel::ContainerDetailsPanel;
pub(crate) use self::container_logs_panel::ContainerLogsPanel;
pub(crate) use self::container_page::ContainerPage;
pub(crate) use self::container_rename_dialog::ContainerRenameDialog;
pub(crate) use self::container_row::ContainerRow;
pub(crate) use self::containers_group::ContainersGroup;
pub(crate) use self::containers_panel::{menu, ContainersPanel};
use crate::window::Window;
use crate::{model, view};

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

fn start(widget: &gtk::Widget, container: &model::Container) {
    container.start(
        clone!(@weak widget => move |result| if let Err(e) = result {
            show_toast(&widget, &gettext("Error on starting container"), e);
        }),
    );
}

fn stop(widget: &gtk::Widget, container: &model::Container) {
    container.stop(
        clone!(@weak widget => move |result| if let Err(e) = result {
            show_toast(&widget, &gettext("Error on stopping container"), e);
        }),
    );
}

fn force_stop(widget: &gtk::Widget, container: &model::Container) {
    container.force_stop(
        clone!(@weak widget => move |result| if let Err(e) = result {
            show_toast(&widget, &gettext("Error on force stopping container"), e);
        }),
    );
}

fn restart(widget: &gtk::Widget, container: &model::Container) {
    container.restart(
        clone!(@weak widget => move |result| if let Err(e) = result {
            show_toast(&widget, &gettext("Error on restarting container"), e);
        }),
    );
}

fn force_restart(widget: &gtk::Widget, container: &model::Container) {
    container.force_restart(
        clone!(@weak widget => move |result| if let Err(e) = result {
            show_toast(&widget, &gettext("Error on force restarting container"), e);
        }),
    );
}

fn pause(widget: &gtk::Widget, container: &model::Container) {
    container.pause(
        clone!(@weak widget => move |result| if let Err(e) = result {
            show_toast(&widget, &gettext("Error on pausing container"), e);
        }),
    );
}

fn resume(widget: &gtk::Widget, container: &model::Container) {
    container.resume(
        clone!(@weak widget => move |result| if let Err(e) = result {
            show_toast(&widget, &gettext("Error on resuming container"), e);
        }),
    );
}

fn commit(widget: &gtk::Widget, container: &model::Container) {
    container.commit(
        clone!(@weak widget => move |result| if let Err(e) = result {
            show_toast(&widget, &gettext("Error on committing container"), e);
        }),
    );
}

fn delete(widget: &gtk::Widget, container: &model::Container) {
    container.delete(
        clone!(@weak widget => move |result| if let Err(e) = result {
            show_toast(&widget, &gettext("Error on deleting container"), e);
        }),
    );
}

fn force_delete(widget: &gtk::Widget, container: &model::Container) {
    container.force_delete(
        clone!(@weak widget => move |result| if let Err(e) = result {
            show_toast(&widget, &gettext("Error on force deleting container"), e);
        }),
    );
}

fn rename(widget: &gtk::Widget, container: Option<model::Container>) {
    let dialog = view::ContainerRenameDialog::from(container);
    dialog.set_transient_for(Some(
        &widget.root().unwrap().downcast::<gtk::Window>().unwrap(),
    ));
    dialog.run_async(clone!(@weak widget => move |dialog, response| {
        on_rename_dialog_response(&widget, dialog.upcast_ref(), response, |widget, dialog| {
            dialog.connect_response(clone!(@weak widget => move |dialog, response| {
                on_rename_dialog_response(&widget, dialog, response, |_, _| {});
            }));
        });
    }));
}

fn on_rename_dialog_response<F>(
    widget: &gtk::Widget,
    dialog: &gtk::Dialog,
    response: gtk::ResponseType,
    op: F,
) where
    F: Fn(&gtk::Widget, &gtk::Dialog),
{
    match response {
        gtk::ResponseType::Cancel | gtk::ResponseType::Apply => dialog.close(),
        _ => op(widget, dialog),
    }
}

fn show_toast(widget: &gtk::Widget, title: &str, e: impl std::error::Error) {
    widget
        .root()
        .unwrap()
        .downcast::<Window>()
        .unwrap()
        .show_toast(
            &adw::Toast::builder()
                .title(&format!("{}: {}", title, e))
                .timeout(3)
                .priority(adw::ToastPriority::High)
                .build(),
        );
}

fn base_menu() -> gio::Menu {
    cascade! {
        gio::Menu::new();
        ..append_section(None, &cascade!{
            gio::Menu::new();
            ..append(Some(&gettext("Re_name…")), Some("container.rename"));
        });
        ..append_section(None, &cascade!{
            gio::Menu::new();
            ..append(Some(&gettext("_Commit")), Some("container.commit"));
        });
        ..append_section(None, &cascade!{
            gio::Menu::new();
            ..append(Some(&gettext("_Delete")), Some("container.delete"));
        });
    }
}

fn stopped_menu() -> gio::Menu {
    cascade! {
        base_menu();
        ..prepend_section(None, &cascade! {
            gio::Menu::new();
            ..append(Some(&gettext("_Start")), Some("container.start"));
        });
    }
}

fn not_stopped_menu() -> gio::Menu {
    cascade! {
        gio::Menu::new();
        ..append(Some(&gettext("S_top")), Some("container.stop"));
        ..append(Some(&gettext("_Force Stop")), Some("container.force-stop"));
        ..append(Some(&gettext("R_estart")), Some("container.restart"));
        ..append(Some(&gettext("F_orce Restart")), Some("container.force-restart"));
    }
}

fn running_menu() -> gio::Menu {
    cascade! {
        base_menu();
        ..prepend_section(None, &cascade! {
            not_stopped_menu();
            ..append(Some(&gettext("_Pause")), Some("container.pause"));
        });
    }
}

fn paused_menu() -> gio::Menu {
    cascade! {
        base_menu();
        ..prepend_section(None, &cascade! {
            not_stopped_menu();
            ..append(Some(&gettext("_Resume")), Some("container.resume"));
        });
    }
}
