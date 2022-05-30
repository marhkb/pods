mod api;
mod application;
#[rustfmt::skip]
mod config;
mod model;
mod search_provider;
mod utils;
mod view;
mod window;

use std::str::FromStr;

use gettextrs::gettext;
use gettextrs::LocaleCategory;
use gtk::gio;
use gtk::glib;
use gtk::prelude::ApplicationExt;
use once_cell::sync::Lazy;

use self::application::Application;
use self::config::GETTEXT_PACKAGE;
use self::config::LOCALEDIR;
use self::config::RESOURCES_FILE;

pub(crate) static RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().unwrap());

pub(crate) static PODMAN: Lazy<api::Podman> =
    Lazy::new(|| api::Podman::unix(glib::user_runtime_dir().join("podman/podman.sock")));

fn main() {
    // Prepare i18n
    gettextrs::setlocale(LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    gettextrs::textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    glib::set_application_name(&gettext("Pods"));

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = setup_cli(Application::default());

    // Command line handling
    app.connect_handle_local_options(|_, dict| {
        if dict.contains("version") {
            // Print version ...
            println!("pods {}", config::VERSION);
            // ... and exit application.
            1
        } else {
            let log_level_filter = match dict.lookup::<String>("log-level").unwrap() {
                Some(level) => {
                    log::LevelFilter::from_str(&level).expect("Error on parsing log-level")
                }
                // Standard log levels if not specified by user
                #[cfg(debug_assertions)]
                None => log::LevelFilter::Debug,
                #[cfg(not(debug_assertions))]
                None => log::LevelFilter::Info,
            };

            if syslog::init_unix(syslog::Facility::LOG_USER, log_level_filter).is_err()
                && syslog::init_unix_custom(
                    syslog::Facility::LOG_USER,
                    log_level_filter,
                    "/run/systemd/journal/dev-log",
                )
                .is_err()
            {
                println!(
                    "Could not initialize logging. \
                    Tried socket paths '/dev/log' and '/run/systemd/journal/dev-log'"
                );
            }

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
