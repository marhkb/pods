mod image_details_page;
mod image_pull_page;
mod image_row;
mod image_row_simple;
mod image_search_response_row;
mod image_search_widget;
mod images_panel;
mod images_prune_page;

use cascade::cascade;
use gettextrs::gettext;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;

pub(crate) use self::image_details_page::ImageDetailsPage;
pub(crate) use self::image_pull_page::ImagePullPage;
pub(crate) use self::image_row::ImageRow;
pub(crate) use self::image_row_simple::ImageRowSimple;
pub(crate) use self::image_search_response_row::ImageSearchResponseRow;
pub(crate) use self::images_panel::ImagesPanel;
pub(crate) use self::images_prune_page::ImagesPrunePage;
use crate::model;
use crate::utils;
use crate::view;

fn create_container(widget: &gtk::Widget, client: &model::Client, image: Option<&model::Image>) {
    utils::find_leaflet_overlay(widget)
        .show_details(&view::ContainerCreationPage::new(client, image));
}

fn delete(widget: &gtk::Widget, image: &model::Image) {
    image.delete(
        clone!(@weak widget => move |image, result| utils::show_toast(&widget, &match result {
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
