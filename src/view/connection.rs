use adw::prelude::*;
use gtk::glib;

use crate::gettext::gettext;
use crate::model;

pub(crate) fn show_ongoing_actions_warning_dialog<W: IsA<gtk::Widget>>(
    widget: &W,
    connection_manager: &model::ConnectionManager,
    heading: &str,
    confirm_label: &str,
) -> bool {
    if connection_manager
        .client()
        .map(|client| client.actions().ongoing() > 0)
        .unwrap_or(false)
    {
        let dialog = adw::AlertDialog::builder()
            .heading(heading)
            .body_use_markup(true)
            .body(gettext(
                "There are ongoing actions whose progress will be irretrievably lost",
            ))
            .build();

        dialog.add_responses(&[("cancel", &gettext("_Cancel")), ("confirm", confirm_label)]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);

        glib::MainContext::default().block_on(dialog.choose_future(Some(widget))) == "confirm"
    } else {
        true
    }
}
