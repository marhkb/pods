mod chooser_page;
mod creator_page;
mod row;
mod switcher_widget;

use adw::prelude::MessageDialogExtManual;
use adw::traits::MessageDialogExt;
use gettextrs::gettext;
use gtk::glib;

pub(crate) use self::chooser_page::ChooserPage;
pub(crate) use self::creator_page::CreatorPage;
pub(crate) use self::row::Row;
pub(crate) use self::switcher_widget::SwitcherWidget;
use crate::model;
use crate::utils;

pub(crate) fn show_ongoing_actions_warning_dialog<W>(
    widget: &W,
    connection_manager: &model::ConnectionManager,
    heading: &str,
) -> bool
where
    W: glib::IsA<gtk::Widget>,
{
    if connection_manager
        .client()
        .map(|client| client.action_list().ongoing() > 0)
        .unwrap_or(false)
    {
        let dialog = adw::MessageDialog::builder()
            .heading(heading)
            .body_use_markup(true)
            .body(&gettext(
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

        glib::MainContext::default().block_on(dialog.run_future()) == "confirm"
    } else {
        true
    }
}
