use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

pub(crate) fn delete_volume_show_confirmation(widget: &gtk::Widget, volume: Option<model::Volume>) {
    if let Some(volume) = volume {
        match volume.container_list().get(0) {
            Some(container) => {
                let dialog = adw::AlertDialog::builder()
                .heading(gettext("Confirm Volume Deletion"))
                .body_use_markup(true)
                .body(gettext!(
                    // Translators: The "{}" is a placeholder for the container name.
                    "Volume is used by container <b>{}</b>. Deleting the volume will also delete these containers.",
                    container.name(),
                ))
                .build();

                dialog.add_responses(&[
                    ("cancel", &gettext("_Cancel")),
                    ("delete", &gettext("_Delete")),
                ]);
                dialog.set_default_response(Some("cancel"));
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

                dialog.choose(
                    widget,
                    gio::Cancellable::NONE,
                    clone!(
                        #[weak]
                        widget,
                        #[weak]
                        volume,
                        move |response| {
                            if response == "delete" {
                                delete_volume(&widget, &volume, true);
                            }
                        }
                    ),
                );
            }
            None => delete_volume(widget, &volume, false),
        }
    }
}

fn delete_volume(widget: &gtk::Widget, volume: &model::Volume, force: bool) {
    volume.delete(
        force,
        clone!(
            #[weak]
            widget,
            move |volume, result| {
                if let Err(e) = result {
                    utils::show_error_toast(
                        &widget,
                        // Translators: The "{}" is a placeholder for the volume name.
                        &gettext!("Error on deleting volume '{}'", &volume.inner().name),
                        &e.to_string(),
                    );
                }
            }
        ),
    );
}

pub(crate) fn create_container(widget: &gtk::Widget, volume: Option<model::Volume>) {
    if let Some(volume) = volume {
        utils::Dialog::new(
            widget,
            view::ContainerCreationPage::from(&volume).upcast_ref(),
        )
        .present();
    }
}
