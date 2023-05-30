#![allow(clippy::format_push_string)]
#![allow(deprecated)]

mod application;
mod podman;
#[rustfmt::skip]
mod config;
mod model;
mod utils;
mod view;
mod widget;
mod window;

use std::path::PathBuf;
use std::str::FromStr;

use gettextrs::gettext;
use gettextrs::LocaleCategory;
use gtk::gio;
use gtk::glib;
use gtk::prelude::ApplicationExt;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;

use self::application::Application;
use self::config::GETTEXT_PACKAGE;
use self::config::LOCALEDIR;
use self::config::RESOURCES_FILE;

pub(crate) static APPLICATION_OPTS: OnceCell<ApplicationOptions> = OnceCell::new();
pub(crate) static RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().unwrap());
pub(crate) static KEYRING: OnceCell<oo7::Keyring> = OnceCell::new();

fn main() {
    glib::log_set_writer_func(glib::log_writer_journald);

    adw::init().expect("Failed to init GTK/libadwaita");
    panel::init();
    sourceview5::init();

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

            APPLICATION_OPTS.set(ApplicationOptions::default()).unwrap();

            -1
        }
    });

    RUNTIME.block_on(async {
        match oo7::Keyring::new().await {
            Ok(keyring) => KEYRING.set(keyring).unwrap(),
            Err(e) => log::error!("Failed to start Secret Service: {e}"),
        }
    });

    app.run();
}

/// Global options for the application
#[derive(Debug)]
pub(crate) struct ApplicationOptions {
    pub(crate) config_dir: PathBuf,
    pub(crate) unix_socket_path: PathBuf,
}
impl Default for ApplicationOptions {
    fn default() -> Self {
        Self {
            config_dir: glib::user_config_dir().join("pods"),
            unix_socket_path: glib::user_runtime_dir().join("podman").join("podman.sock"),
        }
    }
}

fn setup_cli(app: Application) -> Application {
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
