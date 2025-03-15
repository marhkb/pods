use adw::prelude::*;
use gettextrs::gettext;
use glib::clone::Downgrade;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

pub(crate) async fn delete_volume_show_confirmation<W>(widget: &W, volume: Option<&model::Volume>)
where
    W: IsA<gtk::Widget> + Downgrade<Weak = glib::WeakRef<W>>,
{
    let volume = if let Some(volume) = volume {
        volume
    } else {
        return;
    };

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

            if "delete" == dialog.choose_future(widget).await {
                delete_volume(widget, volume, true).await;
            }
        }
        None => delete_volume(widget, volume, false).await,
    }
}

async fn delete_volume<W>(widget: &W, volume: &model::Volume, force: bool)
where
    W: IsA<gtk::Widget> + Downgrade<Weak = glib::WeakRef<W>>,
{
    if let Err(e) = volume.delete(force).await {
        utils::show_error_toast(
            widget,
            // Translators: The "{}" is a placeholder for the volume name.
            &gettext!("Error on deleting volume '{}'", &volume.inner().name),
            &e.to_string(),
        );
    }
}

pub(crate) fn create_container<W: IsA<gtk::Widget>>(widget: &W, volume: Option<model::Volume>) {
    if let Some(volume) = volume {
        utils::Dialog::new(widget, &view::ContainerCreationPage::from(&volume)).present();
    }
}
