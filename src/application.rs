use std::cell::Cell;
use std::cell::OnceCell;
use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use log::debug;
use log::info;

use crate::config;
use crate::view;

mod imp {
    use super::*;

    #[derive(Default)]
    pub(crate) struct Application {
        pub(super) ticks: Cell<u64>,
        pub(super) window: OnceCell<glib::WeakRef<view::Window>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "Application";
        type Type = super::Application;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for Application {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES
                .get_or_init(|| vec![glib::ParamSpecUInt64::builder("ticks").read_only().build()])
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "ticks" => self.obj().ticks().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            glib::timeout_add_seconds_local(
                10,
                clone!(@weak obj => @default-return glib::ControlFlow::Break, move || {
                    obj.tick();
                    glib::ControlFlow::Continue
                }),
            );
        }
    }

    impl ApplicationImpl for Application {
        fn activate(&self) {
            debug!("GtkApplication<Application>::activate");
            self.parent_activate();

            let app = &self.obj();

            if let Some(window) = self.window.get() {
                let window = window.upgrade().unwrap();
                window.present();
                return;
            }

            let window = view::Window::new(app);
            self.window
                .set(window.downgrade())
                .expect("Window already set.");

            app.main_window().present();
        }

        fn startup(&self) {
            debug!("GtkApplication<Application>::startup");
            self.parent_startup();

            let app = &*self.obj();

            // Set icons for shell
            gtk::Window::set_default_icon_name(config::APP_ID);

            app.setup_gactions();
            app.setup_accels();
        }
    }

    impl GtkApplicationImpl for Application {}
    impl AdwApplicationImpl for Application {}
}

glib::wrapper! {
    pub(crate) struct Application(ObjectSubclass<imp::Application>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl Default for Application {
    fn default() -> Self {
        glib::Object::builder()
            .property("application-id", Some(config::APP_ID))
            .property("flags", gio::ApplicationFlags::empty())
            .property("resource-base-path", Some("/com/github/marhkb/Pods/"))
            .build()
    }
}

impl Application {
    fn ticks(&self) -> u64 {
        self.imp().ticks.get()
    }

    fn tick(&self) {
        self.imp().ticks.set(self.ticks() + 1);
        self.notify("ticks");
    }

    pub(super) fn main_window(&self) -> view::Window {
        let imp = self.imp();

        match imp.window.get() {
            Some(window) => window.upgrade().unwrap(),
            None => {
                let window = view::Window::new(self);
                imp.window.set(window.downgrade()).unwrap();
                window
            }
        }
    }

    fn setup_gactions(&self) {
        self.add_action_entries([
            // Quit
            gio::ActionEntry::builder("quit")
                .activate(move |app: &Self, _, _| {
                    // This is needed to trigger the delete event and saving the window state
                    app.main_window().close();
                    app.quit();
                })
                .build(),
            // About
            gio::ActionEntry::builder("about")
                .activate(|app: &Self, _, _| {
                    app.show_about_dialog();
                })
                .build(),
        ]);
    }

    // Sets up keyboard shortcuts
    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<Control>q"]);
    }

    fn show_about_dialog(&self) {
        let dialog = adw::AboutWindow::from_appdata(
            &format!(
                "/com/github/marhkb/Pods/appdata/{}.metainfo.xml",
                config::APP_ID
            ),
            Some(&if config::PROFILE == "Devel" {
                let split = config::VERSION.split('-').collect::<Vec<_>>();
                split[..split.len() - 1].join("~")
            } else {
                config::VERSION.replace('-', "~")
            }),
        );
        dialog.set_transient_for(Some(&self.main_window()));
        dialog.set_version(config::VERSION);
        dialog.set_copyright("© 2022 Marcus Behrendt");
        dialog.set_developers(&[
            "Marcus Behrendt https://github.com/marhkb",
            "Wojciech Kępka https://github.com/vv9k",
        ]);
        dialog.set_designers(&["Marcus Behrendt https://github.com/marhkb"]);
        dialog.set_artists(&[
            "Marcus Behrendt https://github.com/marhkb",
            "Allaeddine Boulefaat https://github.com/allaeddineomc",
            "David Lapshin https://github.com/daudix-UFO",
        ]);
        dialog.set_translator_credits(gettext("translator-credits").as_str());
        dialog.add_credit_section(
            Some(&gettext("Translators")),
            &[
                "Andrea Brandi https://github.com/starise",
                "Óscar Fernández https://github.com/oscfdezdz",
                "rmnscnce https://github.com/rmnscnce",
                "ButterflyOfFire https://github.com/BoFFire",
                "Gustavo Costa https://github.com/xfgusta",
                "Allaeddine Boulefaat https://github.com/allaeddineomc",
                "Gert-dev https://github.com/Gert-dev",
                "Abdelhak AISSAT https://github.com/abdelhak2406",
                "Pierre Thévenet https://github.com/pthevenet",
                "Allan Nordhøy https://github.com/comradekingu",
                "William Gabriel Karvat https://github.com/WgkLink",
            ],
        );

        let controller = gtk::EventControllerKey::new();
        controller.connect_key_pressed(clone!(
            @weak dialog => @default-return glib::Propagation::Stop, move |_, key, _, modifier| {
                if key == gdk::Key::w && modifier == gdk::ModifierType::CONTROL_MASK{
                    dialog.close();
                }
                glib::Propagation::Proceed
            }
        ));
        dialog.add_controller(controller);

        dialog.present();
    }

    pub(crate) fn run(&self) {
        info!("Pods ({})", config::APP_ID);
        info!("Version: {} ({})", config::VERSION, config::PROFILE);
        info!("Datadir: {}", config::PKGDATADIR);

        ApplicationExtManual::run(self);
    }
}
