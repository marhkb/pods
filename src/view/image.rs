use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::clone::Downgrade;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

pub(crate) fn delete_image_show_confirmation<W>(widget: &W, image: Option<model::Image>)
where
    W: IsA<gtk::Widget> + Downgrade<Weak = glib::WeakRef<W>>,
{
    let Some(image) = image else {
        return;
    };

    match image.container_list().get(0) {
        Some(container) => {
            let dialog = adw::AlertDialog::builder()
                    .heading(gettext("Confirm Image Deletion"))
                    .body_use_markup(true)
                    .body(gettext!(
                        // Translators: The "{}" is a placeholder for the container name.
                        "Image is used by container <b>{}</b>. Deleting the image will also delete all its associated containers.",
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
                Some(widget),
                gio::Cancellable::NONE,
                clone!(
                    #[weak]
                    widget,
                    #[weak]
                    image,
                    move |response| {
                        if response == "delete" {
                            remove_image(&widget, &image, true);
                        }
                    }
                ),
            );
        }
        None => remove_image(widget, &image, false),
    }
}

pub(crate) fn remove_image<W>(widget: &W, image: &model::Image, force: bool)
where
    W: IsA<gtk::Widget> + Downgrade<Weak = glib::WeakRef<W>>,
{
    image.remove(
        force,
        clone!(
            #[weak]
            widget,
            move |image, result| {
                if let Err(e) = result {
                    utils::show_error_toast(
                        &widget,
                        // Translators: The "{}" is a placeholder for the image id.
                        &gettext!("Error on removing image '{}'", image.id()),
                        &e.to_string(),
                    );
                }
            }
        ),
    );
}

pub(crate) fn create_container<W: IsA<gtk::Widget>>(widget: &W, image: Option<model::Image>) {
    if let Some(image) = image {
        utils::Dialog::new(widget, &view::ContainerCreationPage::from(&image)).present();
    }
}
