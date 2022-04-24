mod image_details_page;
mod image_row;
mod image_row_simple;
mod images_panel;
mod images_prune_dialog;

use cascade::cascade;
use gettextrs::gettext;
use gtk::glib::clone;
use gtk::prelude::{Cast, DialogExtManual};
use gtk::traits::{GtkWindowExt, WidgetExt};
use gtk::{gio, glib};

pub(crate) use self::image_details_page::ImageDetailsPage;
pub(crate) use self::image_row::ImageRow;
pub(crate) use self::image_row_simple::ImageRowSimple;
pub(crate) use self::images_panel::ImagesPanel;
pub(crate) use self::images_prune_dialog::ImagesPruneDialog;
use crate::{model, utils, view, PODMAN};

fn create_container<T>(widget: &gtk::Widget, from: &T)
where
    view::ContainerCreationDialog: for<'a> From<Option<&'a T>>,
{
    let dialog = view::ContainerCreationDialog::from(Some(from));
    dialog.set_transient_for(Some(
        &widget.root().unwrap().downcast::<gtk::Window>().unwrap(),
    ));
    dialog.run_async(clone!(@weak widget => move |dialog, response| {
        if let gtk::ResponseType::Other(1) = response {
            let id = dialog.created_container_id().unwrap().to_owned();
            utils::do_async(
                async move { PODMAN.containers().get(id).start(None).await },
                clone!(@weak widget => move |result| {
                    if let Err(e) = result {
                        super::show_toast(
                            &widget,
                            &format!("Failed to start container: {}", e)
                        )
                    }
                }),
            );
        }
        dialog.close();
    }));
}

fn delete(widget: &gtk::Widget, image: &model::Image) {
    image.delete(
        clone!(@weak widget => move |image, result| super::show_toast(&widget, &match result {
            Ok(_) => {
                // Translators: "{}" is a placeholder for the image id.
                gettext!("Successfully deleted image '{}'", image.id())
            }
            Err(_) => {
                // Translators: "{}" is a placeholder for the image id.
                gettext!("Error on deleting image '{}'", image.id())
            }
        })),
    );
}

pub(crate) fn menu() -> gio::Menu {
    cascade! {
        gio::Menu::new();
        ..append_section(None, &cascade!{
            gio::Menu::new();
            ..append(Some(&gettext("_Show intermediate images")), Some("images.show-intermediates"));
        });
        ..append_section(None, &cascade!{
            gio::Menu::new();
            ..append(Some(&gettext("_Prune unused images…")), Some("images.prune-unused"));
        });
    }
}

fn image_menu() -> gio::Menu {
    cascade! {
        gio::Menu::new();
        ..append_section(None, &cascade!{
            gio::Menu::new();
            ..append(Some(&gettext("_Create Container…")), Some("image.create-container"));
        });
        ..append_section(None, &cascade!{
            gio::Menu::new();
            ..append(Some(&gettext("_Delete")), Some("image.delete"));
        });
    }
}
