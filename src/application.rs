use std::cell::Cell;

use adw::subclass::prelude::AdwApplicationImpl;
use gettextrs::gettext;
use glib::clone;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use log::debug;
use log::info;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;

use crate::config;
use crate::window::Window;

mod imp {
    use super::*;

    #[derive(Default)]
    pub(crate) struct Application {
        pub(super) ticks: Cell<u64>,
        pub(super) window: OnceCell<glib::WeakRef<Window>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "Application";
        type Type = super::Application;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for Application {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecUInt64::builder("ticks")
                    .flags(glib::ParamFlags::READABLE)
                    .build()]
            });

            PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "ticks" => self.instance().ticks().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            glib::timeout_add_seconds_local(
                10,
                clone!(@weak obj => @default-return glib::Continue(false), move || {
                    obj.tick();
                    glib::Continue(true)
                }),
            );
        }
    }

    impl ApplicationImpl for Application {
        fn activate(&self) {
            debug!("GtkApplication<Application>::activate");
            self.parent_activate();

            let app = &self.instance();

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

        fn startup(&self) {
            debug!("GtkApplication<Application>::startup");
            self.parent_startup();

            let app = &*self.instance();

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
        glib::Object::builder::<Self>()
            .property("application-id", &Some(config::APP_ID))
            .property("flags", &gio::ApplicationFlags::empty())
            .property("resource-base-path", &Some("/com/github/marhkb/Pods/"))
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

    pub(super) fn main_window(&self) -> Window {
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
        ])
        .unwrap();
    }

    // Sets up keyboard shortcuts
    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<Control>q"]);
    }

    fn show_about_dialog(&self) {
        let about = adw::AboutWindow::builder()
            .transient_for(&self.main_window())
            .application_name("Pods")
            .application_icon(config::APP_ID)
            .version(config::VERSION)
            .website("https://github.com/marhkb/pods/")
            .issue_url("https://github.com/marhkb/pods/issues")
            .developer_name("Marcus Behrendt")
            .copyright("© 2022 Marcus Behrendt")
            .license_type(gtk::License::Gpl30)
            .developers(vec![
                "Marcus Behrendt https://github.com/marhkb".into(),
                "Wojciech Kępka https://github.com/vv9k".into(),
            ])
            .designers(vec!["Marcus Behrendt https://github.com/marhkb".into()])
            .artists(vec![
                "Marcus Behrendt https://github.com/marhkb".into(),
                "Allaeddine Boulefaat https://github.com/allaeddineomc".into(),
            ])
            .translator_credits(gettext("translator-credits").as_str())
            .build();

        about.add_credit_section(
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

        about.present();
    }

    pub(crate) fn run(&self) {
        info!("Pods ({})", config::APP_ID);
        info!("Version: {} ({})", config::VERSION, config::PROFILE);
        info!("Datadir: {}", config::PKGDATADIR);

        ApplicationExtManual::run(self);
    }
}
