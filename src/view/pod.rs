use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::clone::Downgrade;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

pub(crate) fn pod_status_css_class(status: model::PodStatus) -> &'static str {
    use model::PodStatus::*;

    match status {
        Running => "pod-status-running",
        Degraded => "pod-status-degraded",
        Unknown => "pod-status-unknown",
        _ => "pod-status-not-running",
    }
}

macro_rules! pod_action {
    (fn $name:ident => $action:ident($($param:literal),*) => $error:tt) => {
        pub(crate) fn $name<W>(widget: &W, pod: Option<crate::model::Pod>)
        where
            W: gtk::glib::prelude::IsA<gtk::Widget> + gtk::glib::clone::Downgrade<Weak = gtk::glib::WeakRef<W>>,
        {
            if let Some(pod) = pod {
                pod.$action(
                    $($param,)*
                    gtk::glib::clone!(#[weak] widget, move |result| if let Err(e) = result {
                        crate::utils::show_error_toast(&widget, &$error, &e.to_string());
                    }),
                );
            }
        }
    };
}

pod_action!(fn start => start() => { gettextrs::gettext("Error on starting pod") });
pod_action!(fn stop => stop(false) => { gettextrs::gettext("Error on stopping pod") });
pod_action!(fn kill => stop(true) => { gettextrs::gettext("Error on killing pod") });
pod_action!(fn restart => restart(false) => { gettextrs::gettext("Error on restarting pod") });
pod_action!(fn pause => pause() => { gettextrs::gettext("Error on pausing pod") });
pod_action!(fn resume => resume() => { gettextrs::gettext("Error on resuming pod") });
pod_action!(fn delete => delete(false) => { gettextrs::gettext("Error on deleting pod") });

pub(crate) fn show_delete_confirmation_dialog<W>(widget: &W, pod: Option<model::Pod>)
where
    W: IsA<gtk::Widget> + Downgrade<Weak = glib::WeakRef<W>>,
{
    if let Some(pod) = pod {
        match pod.container_list().first_non_infra() {
            Some(container) => {
                let dialog = adw::AlertDialog::builder()
                .heading(gettext("Confirm Pod Deletion"))
                .body_use_markup(true)
                .body(gettext!(
                    // Translators: The "{}" is a placeholder for the container name.
                    "Pod contains container <b>{}</b>. Deleting the pod will also delete all its containers.",
                    container.name()
                ))
                .build();

                dialog.add_responses(&[
                    ("cancel", &gettext("_Cancel")),
                    ("delete", &gettext("_Delete")),
                ]);
                dialog.set_default_response(Some("cancel"));
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

                dialog.choose(
                    widget,
                    gio::Cancellable::NONE,
                    clone!(
                        #[weak]
                        widget,
                        move |response| {
                            if response == "delete" {
                                delete(&widget, Some(pod));
                            }
                        }
                    ),
                );
            }
            None => delete(widget, Some(pod)),
        }
    }
}

pub(crate) fn create_container<W: IsA<gtk::Widget>>(widget: &W, pod: Option<model::Pod>) {
    if let Some(pod) = pod {
        utils::Dialog::new(widget, &view::ContainerCreationPage::from(&pod)).present();
    }
}
