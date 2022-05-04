use std::io;

use futures::future;
use futures::FutureExt;
use gettextrs::gettext;
use glib::clone;
use glib::WeakRef;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use log::debug;
use log::info;
use once_cell::sync::OnceCell;

use crate::config;
use crate::utils;
use crate::window::Window;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct Application {
        pub(super) window: OnceCell<WeakRef<Window>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "Application";
        type Type = super::Application;
        type ParentType = gtk::Application;
    }

    impl ObjectImpl for Application {}

    impl ApplicationImpl for Application {
        fn activate(&self, app: &Self::Type) {
            debug!("GtkApplication<Application>::activate");
            self.parent_activate(app);

            if let Some(window) = self.window.get() {
                let window = window.upgrade().unwrap();
                window.present();
                return;
            }

            let window = Window::new(app);
            self.window
                .set(window.downgrade())
                .expect("Window already set.");

            app.main_window().present();
        }

        fn startup(&self, app: &Self::Type) {
            debug!("GtkApplication<Application>::startup");
            self.parent_startup(app);

            // Set icons for shell
            gtk::Window::set_default_icon_name(config::APP_ID);

            app.setup_css();
            app.setup_gactions();
            app.setup_accels();
        }
    }

    impl GtkApplicationImpl for Application {}
}

glib::wrapper! {
    pub(crate) struct Application(ObjectSubclass<imp::Application>)
        @extends gio::Application, gtk::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl Default for Application {
    fn default() -> Self {
        glib::Object::new(&[
            ("application-id", &Some(config::APP_ID)),
            ("flags", &gio::ApplicationFlags::empty()),
            ("resource-base-path", &Some("/com/github/marhkb/Pods/")),
        ])
        .expect("Application initialization failed...")
    }
}

impl Application {
    fn main_window(&self) -> Window {
        self.imp().window.get().unwrap().upgrade().unwrap()
    }

    fn setup_gactions(&self) {
        // Quit
        let action_quit = gio::SimpleAction::new("quit", None);
        action_quit.connect_activate(clone!(@weak self as app => move |_, _| {
            // This is needed to trigger the delete event and saving the window state
            app.main_window().close();
            app.quit();
        }));
        self.add_action(&action_quit);

        // About
        let action_about = gio::SimpleAction::new("about", None);
        action_about.connect_activate(clone!(@weak self as app => move |_, _| {
            app.show_about_dialog();
        }));
        self.add_action(&action_about);

        // Start podman user service
        let action_start_service = gio::SimpleAction::new("start-service", None);
        action_start_service.connect_activate(clone!(@weak self as app => move |action, _| {
            action.set_enabled(false);
            app.start_service(action);
        }));
        self.add_action(&action_start_service);
    }

    // Sets up keyboard shortcuts
    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<Control>q"]);
    }

    fn setup_css(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/com/github/marhkb/Pods/style.css");
        if let Some(display) = gdk::Display::default() {
            gtk::StyleContext::add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    fn show_about_dialog(&self) {
        let dialog = gtk::AboutDialog::builder()
            .program_name("Pods")
            .logo_icon_name(config::APP_ID)
            .license_type(gtk::License::Gpl30)
            .website("https://github.com/marhkb/pods/")
            .version(config::VERSION)
            .transient_for(&self.main_window())
            .translator_credits(&gettext("translator-credits"))
            .modal(true)
            .authors(vec!["Marcus Behrendt".into()])
            .artists(vec![
                "Marcus Behrendt".into(),
                "Allaeddine Boulefaat".into(),
            ])
            .build();

        dialog.present();
    }

    fn start_service(&self, action: &gio::SimpleAction) {
        utils::do_async(
            future::try_select(
                start_service_assuming_flatpak().boxed(),
                start_service_assuming_native().boxed(),
            ),
            clone!(@weak self as obj, @weak action => move |result| match result {
                Ok(future::Either::Left((output, _)))
                | Ok(future::Either::Right((output, _))) => {
                    obj.on_service_start_command_issued(output, &action);
                },
                Err(future::Either::Left((e1, f))) | Err(future::Either::Right((e1, f))) => {
                    utils::do_async(f, clone!(@weak obj, @weak action => move |result| match result {
                        Ok(output) => obj.on_service_start_command_issued(output, &action),
                        Err(e2) => {
                            action.set_enabled(true);

                            log::error!(
                                "Failed to execute command to start Podman. \
                                Neither flatpak nor native method worked.\n\t{e1}\n\t{e2}"
                            );
                            obj.main_window().show_toast(
                                &adw::Toast::builder()
                                    .title(
                                        &gettext("Failed to execute command to start Podman")
                                    )
                                    .timeout(3)
                                    .priority(adw::ToastPriority::High)
                                    .build(),
                            );
                        }
                    }));
                },
            }),
        );
    }

    fn on_service_start_command_issued(
        &self,
        output: std::process::Output,
        action: &gio::SimpleAction,
    ) {
        action.set_enabled(true);

        if output.status.success() {
            self.main_window().check_service();
        } else {
            log::error!(
                "command to start Podman returned exit code: {}",
                output.status
            );
            self.main_window().show_toast(
                &adw::Toast::builder()
                    .title(&gettext!(
                        // Translators: "{}" is the placeholder for the exit code.
                        "Command to start Podman returned exit code: {}",
                        output.status
                    ))
                    .timeout(3)
                    .priority(adw::ToastPriority::High)
                    .build(),
            );
        }
    }

    pub(crate) fn run(&self) {
        info!("Pods ({})", config::APP_ID);
        info!("Version: {} ({})", config::VERSION, config::PROFILE);
        info!("Datadir: {}", config::PKGDATADIR);

        ApplicationExtManual::run(self);
    }
}

async fn start_service_assuming_flatpak() -> Result<std::process::Output, io::Error> {
    tokio::process::Command::new("flatpak-spawn")
        .args(&[
            "--host",
            "systemctl",
            "--user",
            "enable",
            "--now",
            "podman.socket",
        ])
        .output()
        .await
}

async fn start_service_assuming_native() -> Result<std::process::Output, io::Error> {
    tokio::process::Command::new("systemctl")
        .args(&["--user", "enable", "--now", "podman.socket"])
        .output()
        .await
}
