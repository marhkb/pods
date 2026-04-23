#![allow(clippy::format_push_string)]
#![allow(deprecated)]

mod application;
#[rustfmt::skip]
mod config;
mod engine;
mod model;
mod rt;
mod utils;
mod view;
mod widget;

use std::ops::ControlFlow;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::OnceLock;

use adw::prelude::*;
use gettextrs::LocaleCategory;
use gettextrs::gettext;
use glib::clone;
use gtk::gio;
use gtk::glib;
use smart_default::SmartDefault;

use self::application::Application;

pub(crate) static APPLICATION_OPTS: OnceLock<ApplicationOptions> = OnceLock::new();
pub(crate) static KEYRING: OnceLock<oo7::Keyring> = OnceLock::new();

fn main() {
    let app = setup_cli(Application::default());

    // Command line handling
    app.connect_handle_local_options(|_, dict| {
        if dict.contains("version") {
            // Print version ...
            println!("pods {}", config::VERSION);
            // ... and exit application.
            ControlFlow::Break(glib::ExitCode::new(0))
        } else {
            let (log_level_filter, log_level) = match dict.lookup::<String>("log-level").unwrap() {
                Some(level) => (
                    log::LevelFilter::from_str(&level).expect("Error on parsing log-level-filter"),
                    log::Level::from_str(&level).expect("Error on parsing log-level"),
                ),
                // Standard log levels if not specified by user
                #[cfg(debug_assertions)]
                None => (log::LevelFilter::Debug, log::Level::Debug),
                #[cfg(not(debug_assertions))]
                None => (log::LevelFilter::Info, log::Level::Info),
            };

            let mut loggers: Vec<Box<dyn log::Log>> = Vec::new();

            let term_logger = simplelog::TermLogger::new(
                log_level_filter,
                simplelog::ConfigBuilder::new()
                    .add_filter_ignore_str("bollard")
                    .build(),
                simplelog::TerminalMode::Mixed,
                simplelog::ColorChoice::Auto,
            );
            loggers.push(term_logger);

            let syslog_formatter = syslog::Formatter3164 {
                facility: syslog::Facility::LOG_USER,
                hostname: None,
                process: String::from("pods"),
                pid: std::process::id(),
            };
            match syslog::unix(syslog_formatter.clone())
                .or_else(|_| syslog::unix_custom(syslog_formatter, "/run/systemd/journal/dev-log"))
            {
                Ok(logger) => {
                    loggers.push(Box::new(syslog::BasicLogger::new(logger)));
                }
                Err(_) => println!("Could not initialize syslog logging."),
            }

            multi_log::MultiLogger::init(loggers, log_level).unwrap();

            glib::log_set_writer_func(clone!(
                #[strong]
                log_level,
                move |glib_log_level, fields| {
                    if (glib_log_level == glib::LogLevel::Debug && log_level >= log::Level::Debug)
                        || (glib_log_level == glib::LogLevel::Info && log_level >= log::Level::Info)
                        || (glib_log_level == glib::LogLevel::Message
                            && log_level >= log::Level::Info)
                        || (glib_log_level == glib::LogLevel::Warning
                            && log_level >= log::Level::Warn)
                        || (glib_log_level == glib::LogLevel::Error
                            && log_level >= log::Level::Error)
                        || (glib_log_level == glib::LogLevel::Critical
                            && log_level >= log::Level::Error)
                    {
                        glib::log_writer_standard_streams(glib_log_level, fields);
                        glib::log_writer_journald(glib_log_level, fields);

                        glib::LogWriterOutput::Handled
                    } else {
                        glib::LogWriterOutput::Unhandled
                    }
                }
            ));

            adw::init().expect("Failed to init GTK/libadwaita");
            sourceview5::init();
            crate::init();

            // Prepare i18n
            gettextrs::setlocale(LocaleCategory::LcAll, "");
            gettextrs::bindtextdomain(config::GETTEXT_PACKAGE, config::LOCALEDIR)
                .expect("Unable to bind the text domain");
            gettextrs::textdomain(config::GETTEXT_PACKAGE)
                .expect("Unable to switch to the text domain");

            glib::set_application_name(&gettext("Pods"));

            gio::resources_register(
                &gio::Resource::load(config::RESOURCES_FILE)
                    .expect("Could not load gresource file"),
            );
            gio::resources_register(
                &gio::Resource::load(config::APPDATA_RESOURCES_FILE)
                    .expect("Could not load gresource file"),
            );
            gio::resources_register(
                &gio::Resource::load(config::UI_RESOURCES_FILE)
                    .expect("Could not load UI gresource file"),
            );

            APPLICATION_OPTS.set(ApplicationOptions::default()).unwrap();

            ControlFlow::Continue(())
        }
    });

    rt::Promise::new(async {
        match oo7::Keyring::new().await {
            Ok(keyring) => KEYRING.set(keyring).unwrap(),
            Err(e) => log::error!("Failed to start Secret Service: {e}"),
        }
    })
    .block_on();

    app.run();
}

/// Global options for the application
#[derive(Debug, SmartDefault)]
pub(crate) struct ApplicationOptions {
    #[default(glib::user_config_dir().join("pods"))]
    pub(crate) config_dir: PathBuf,
    #[default(glib::user_runtime_dir().join("podman").join("podman.sock"))]
    pub(crate) unix_socket_path: PathBuf,
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

fn init() {
    model::init_gobjects();
    view::init_gobjects();
    widget::init_gobjects();
}
