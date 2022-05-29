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
use search_provider::SearchProvider;
use search_provider::SearchProviderImpl;
use serde::Deserialize;
use serde::Serialize;

use crate::api;
use crate::config;
use crate::utils;
use crate::window::Window;
use crate::PODMAN;
use crate::RUNTIME;

mod imp {
    use super::*;

    #[derive(Default)]
    pub(crate) struct Application {
        pub(super) window: OnceCell<WeakRef<Window>>,
        pub(super) search_provider: OnceCell<SearchProvider<super::Application>>,
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
            app.setup_search_provider();
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
        let imp = self.imp();

        match imp.window.get() {
            Some(window) => window.upgrade().unwrap(),
            None => {
                let window = Window::new(self);
                imp.window.set(window.downgrade()).unwrap();
                window
            }
        }
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

    fn setup_search_provider(&self) {
        glib::MainContext::default().spawn_local(clone!(@weak self as app => async move {
            match SearchProvider::new(
                app.clone(),
                format!("{}.SearchProvider", config::APP_ID),
                config::OBJECT_PATH,
            )
            .await
            {
                Ok(search_provider) => {
                    if app.imp().search_provider.set(search_provider).is_err() {
                        unreachable!("Search provider already set");
                    }
                }
                Err(err) => log::debug!("Could not start search provider: {}", err),
            }
        }));
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

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ResultId {
    id: String,
    kind: SearchResultKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SearchResultMeta {
    id: String,
    name: String,
    description: String,
    kind: SearchResultKind,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum SearchResultKind {
    Image,
    Container,
}

impl SearchProviderImpl for Application {
    fn activate_result(
        &self,
        identifier: search_provider::ResultID,
        _terms: &[String],
        timestamp: u32,
    ) {
        let window = self.main_window();
        window.present_with_time(timestamp);

        let search_result = serde_json::from_str::<ResultId>(&identifier).unwrap();
        match search_result.kind {
            SearchResultKind::Image => window.show_image_details(search_result.id),
            SearchResultKind::Container => window.show_container_details(search_result.id),
        }
    }

    fn initial_result_set(&self, terms: &[String]) -> Vec<search_provider::ResultID> {
        let (images, containers) = RUNTIME.block_on(future::join(
            PODMAN
                .images()
                .list(&api::ImageListOpts::builder().all(true).build()),
            PODMAN
                .containers()
                .list(&api::ContainerListOpts::builder().all(true).build()),
        ));

        let terms_lower = terms.iter().map(|s| s.to_lowercase()).collect::<Vec<_>>();

        images
            .map(|images| {
                images.into_iter().filter_map(|image| {
                    let id = image.id.unwrap();
                    let tag =
                        utils::format_option(image.repo_tags.and_then(|r| r.first().cloned()));

                    let id_lower = id.to_lowercase();
                    let tag_lower = tag.to_lowercase();

                    if terms_lower
                        .iter()
                        .any(|term| id_lower.contains(term) || tag_lower.contains(term))
                    {
                        Some(SearchResultMeta {
                            id,
                            name: tag,
                            description: gettext!(
                                // Translators: "{}" is the placeholder for the amount of containers.
                                "{} containers",
                                image.containers.unwrap_or_default()
                            ),
                            kind: SearchResultKind::Image,
                        })
                    } else {
                        None
                    }
                })
            })
            .into_iter()
            .flatten()
            .chain(
                containers
                    .map(|containers| {
                        containers.into_iter().filter_map(|container| {
                            let id = container.id.unwrap();
                            let name = container.names.unwrap().pop().unwrap();

                            let id_lower = id.to_lowercase();
                            let name_lower = name.to_lowercase();

                            if terms_lower
                                .iter()
                                .any(|term| id_lower.contains(term) || name_lower.contains(term))
                            {
                                Some(SearchResultMeta {
                                    id,
                                    name,
                                    description: container.image.unwrap(),
                                    kind: SearchResultKind::Container,
                                })
                            } else {
                                None
                            }
                        })
                    })
                    .into_iter()
                    .flatten(),
            )
            .map(|search_result| serde_json::to_string(&search_result).unwrap())
            .collect()
    }

    fn result_metas(
        &self,
        identifiers: &[search_provider::ResultID],
    ) -> Vec<search_provider::ResultMeta> {
        identifiers
            .iter()
            .map(|s| serde_json::from_str::<SearchResultMeta>(s).unwrap())
            .map(|search_result| {
                search_provider::ResultMeta::builder(
                    serde_json::to_string(&ResultId {
                        id: search_result.id,
                        kind: search_result.kind,
                    })
                    .unwrap(),
                    &search_result.name,
                )
                .description(&search_result.description)
                .gicon(match search_result.kind {
                    SearchResultKind::Image => "image-x-generic-symbolic",
                    SearchResultKind::Container => "package-x-generic-symbolic",
                })
                .build()
            })
            .collect()
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
