#![allow(clippy::format_push_string)]
#![allow(deprecated)]

mod application;
mod podman;
#[rustfmt::skip]
mod config;
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
                simplelog::Config::default(),
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

fn init() {
    model::Connection::static_type();

    view::ActionPage::static_type();
    view::ActionRow::static_type();
    view::ActionsButton::static_type();
    view::ActionsSidebar::static_type();
    view::ClientView::static_type();
    view::ConnectionChooserPage::static_type();
    view::ConnectionCustomInfoDialog::static_type();
    view::ConnectionRow::static_type();
    view::ConnectionsSidebar::static_type();
    view::ContainerCard::static_type();
    view::ContainerCommitPage::static_type();
    view::ContainerFilesGetPage::static_type();
    view::ContainerFilesPutPage::static_type();
    view::ContainerHealthCheckLogRow::static_type();
    view::ContainerHealthCheckPage::static_type();
    view::ContainerLogPage::static_type();
    view::ContainerMenuButton::static_type();
    view::ContainerPropertiesGroup::static_type();
    view::ContainerResources::static_type();
    view::ContainerTerminal::static_type();
    view::ContainerTerminalPage::static_type();
    view::ContainerVolumeRow::static_type();
    view::ContainersCountBar::static_type();
    view::ContainersGridView::static_type();
    view::ContainersGroup::static_type();
    view::ContainersListView::static_type();
    view::ContainersPanel::static_type();
    view::ContainersPrunePage::static_type();
    view::ContainersRow::static_type();
    view::ImageBuildPage::static_type();
    view::ImageHistoryPage::static_type();
    view::ImageMenuButton::static_type();
    view::ImagePullPage::static_type();
    view::ImageSearchPage::static_type();
    view::ImageSearchResponseRow::static_type();
    view::ImageSelectionComboRow::static_type();
    view::ImageSelectionPage::static_type();
    view::ImagesPanel::static_type();
    view::ImagesRow::static_type();
    view::InfoPanel::static_type();
    view::InfoRow::static_type();
    view::NetworksPanel::static_type();
    view::NetworksRow::static_type();
    view::NetworksRow::static_type();
    view::PodMenuButton::static_type();
    view::PodRow::static_type();
    view::PodSelectionPage::static_type();
    view::PodsPanel::static_type();
    view::PodsPrunePage::static_type();
    view::PodsRow::static_type();
    view::RepoTagAddDialog::static_type();
    view::RepoTagPushPage::static_type();
    view::RepoTagRow::static_type();
    view::RepoTagSelectionPage::static_type();
    view::RepoTagSimpleRow::static_type();
    view::ScalableTextViewPage::static_type();
    view::SearchPanel::static_type();
    view::VolumeRow::static_type();
    view::VolumesGroup::static_type();
    view::VolumesPanel::static_type();
    view::VolumesPrunePage::static_type();
    view::VolumesRow::static_type();
    view::WelcomePage::static_type();
    view::Window::static_type();

    widget::CircularProgressBar::static_type();
    widget::CountBadge::static_type();
    widget::DateTimeRow::static_type();
    widget::MainMenuButton::static_type();
    widget::PropertyRow::static_type();
    widget::PropertyWidgetRow::static_type();
    widget::RandomNameEntryRow::static_type();
    widget::ScalableTextView::static_type();
    widget::SourceViewSearchWidget::static_type();
    widget::Spinner::static_type();
    widget::TextSearchEntry::static_type();
    widget::ZoomControl::static_type();
}
