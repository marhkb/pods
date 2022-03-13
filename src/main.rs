mod api;
mod application;
#[rustfmt::skip]
mod config;
mod model;
mod utils;
mod view;
mod window;

use std::str::FromStr;

use gettextrs::{gettext, LocaleCategory};
use gtk::prelude::ApplicationExt;
use gtk::{gio, glib};
use log::LevelFilter;
use once_cell::sync::Lazy;
use syslog::Facility;

use self::application::Application;
use self::config::{GETTEXT_PACKAGE, LOCALEDIR, RESOURCES_FILE};

pub(crate) static RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().unwrap());

pub(crate) static PODMAN: Lazy<api::Podman> =
    Lazy::new(|| api::Podman::unix(glib::user_runtime_dir().join("podman/podman.sock")));

fn main() {
    // Prepare i18n
    gettextrs::setlocale(LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    gettextrs::textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    glib::set_application_name(&gettext("Symphony"));

    gtk::init().expect("Unable to start GTK4");
    adw::init();

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = setup_cli(Application::default());

    // Command line handling
    app.connect_handle_local_options(|_, dict| {
        if dict.contains("version") {
            // Print version ...
            println!("symphony {}", config::VERSION);
            // ... and exit application.
            1
        } else {
            let log_level_filter = match dict.lookup::<String>("log-level").unwrap() {
                Some(level) => LevelFilter::from_str(&level).expect("Error on parsing log-level"),
                // Standard log levels if not specified by user
                None => LevelFilter::Warn,
            };

            syslog::init(Facility::LOG_USER, log_level_filter, Some("rodman"))
                .expect("could not initialize logging");

            -1
        }
    });

    app.run();
}

fn setup_cli<A: glib::IsA<gio::Application>>(app: A) -> A {
    app.add_main_option(
        "version",
        b'v'.into(),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        &gettext("Prints application version"),
        None,
    );

    app.add_main_option(
        "log-level",
        b'l'.into(),
        glib::OptionFlags::NONE,
        glib::OptionArg::String,
        &gettext("Specify the minimum log level"),
        Some("error|warn|info|debug|trace"),
    );

    app
}
