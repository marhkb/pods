use adw::prelude::*;
use gettextrs::gettext;
use glib::clone::Downgrade;
use gtk::glib;

use crate::model;
use crate::utils;

pub(crate) async fn delete_show_confirmation<W>(widget: &W, network: Option<&model::Network>)
where
    W: IsA<gtk::Widget> + Downgrade<Weak = glib::WeakRef<W>>,
{
    let network = if let Some(network) = network {
        network
    } else {
        return;
    };

    match network.container_list().get(0) {
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
                delete(widget, network, true).await;
            }
        }
        None => delete(widget, network, false).await,
    }
}

async fn delete<W>(widget: &W, network: &model::Network, force: bool)
where
    W: IsA<gtk::Widget> + Downgrade<Weak = glib::WeakRef<W>>,
{
    if let Err(e) = network.delete(force).await {
        utils::show_error_toast(
            widget,
            // Translators: The "{}" is a placeholder for the network name.
            &gettext!("Error on deleting network '{}'", &network.inner().name.as_ref().unwrap()),
            &e.to_string(),
        );
    }
}
