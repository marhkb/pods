use adw::prelude::*;
use gettextrs::gettext;
use gtk::glib;

use crate::model;
use crate::utils;

pub(crate) fn show_ongoing_actions_warning_dialog<W: IsA<gtk::Widget>>(
    widget: &W,
    connection_manager: &model::ConnectionManager,
    heading: &str,
) -> bool {
    if connection_manager
        .client()
        .map(|client| client.action_list().ongoing() > 0)
        .unwrap_or(false)
    {
        let dialog = adw::MessageDialog::builder()
            .heading(heading)
            .body_use_markup(true)
            .body(gettext(
                "There are ongoing actions whose progress will be irretrievably lost",
            ))
            .transient_for(&utils::root(widget))
            .build();

        dialog.add_responses(&[
            ("cancel", &gettext("_Cancel")),
            ("confirm", &gettext("_Confirm")),
        ]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);

        glib::MainContext::default().block_on(dialog.choose_future()) == "confirm"
    } else {
        true
    }
}
