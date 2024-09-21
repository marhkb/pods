use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

pub(crate) fn delete_image_show_confirmation(widget: &gtk::Widget, image: Option<model::Image>) {
    if let Some(image) = image {
        let first_container = image.container_list().get(0);

        if image.containers() > 0 || first_container.is_some() {
            let dialog = adw::MessageDialog::builder()
                .heading(gettext("Confirm Forced Image Deletion"))
                .body_use_markup(true)
                .body(
                    match first_container.as_ref().map(|c| c.name()) {
                        Some(id) => gettext!(
                            // Translators: The "{}" is a placeholder for the container name.
                            "Image is used by container <b>{}</b>. Deleting the image will also delete all its associated containers.",
                            id
                        ),
                        None => gettext(
                           "Image is used by a container. Deleting the image will also delete all its associated containers.",
                       ),
                    }

                )
                .modal(true)
                .transient_for(&utils::root(widget)).build();

            dialog.add_responses(&[
                ("cancel", &gettext("_Cancel")),
                ("delete", &gettext("_Force Delete")),
            ]);
            dialog.set_default_response(Some("cancel"));
            dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

            dialog.choose(
                gio::Cancellable::NONE,
                clone!(@weak widget, @weak image => move |response| {
                    if response == "delete" {
                        delete_image(&widget, &image);
                    }
                }),
            );
        } else {
            delete_image(widget, &image);
        }
    }
}

pub(crate) fn delete_image(widget: &gtk::Widget, image: &model::Image) {
    image.delete(clone!(@weak widget => move |image, result| {
        if let Err(e) = result {
            utils::show_error_toast(
                &widget,
                // Translators: The "{}" is a placeholder for the image id.
                &gettext!("Error on deleting image '{}'", image.id()),
                &e.to_string(),
            );
        }
    }));
}

pub(crate) fn create_container(widget: &gtk::Widget, image: Option<model::Image>) {
    if let Some(image) = image {
        utils::show_dialog(
            widget,
            view::ContainerCreationPage::from(&image).upcast_ref(),
        );
    }
}
