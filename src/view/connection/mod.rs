mod chooser_page;
mod creation_page;
mod custom_info_dialog;
mod row;
mod sidebar;
mod switcher;

use adw::prelude::MessageDialogExtManual;
use adw::traits::MessageDialogExt;
use gettextrs::gettext;
use gtk::glib;

pub(crate) use self::chooser_page::ChooserPage;
pub(crate) use self::creation_page::CreationPage;
pub(crate) use self::custom_info_dialog::CustomInfoDialog;
pub(crate) use self::row::Row;
pub(crate) use self::sidebar::Sidebar;
pub(crate) use self::switcher::Switcher;
use crate::model;
use crate::utils;

pub(crate) fn show_ongoing_actions_warning_dialog(
    widget: &gtk::Widget,
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
                "There are ongoing actions whose progress will be irretrievably lost.",
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
