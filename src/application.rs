use std::borrow::Cow;

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
use crate::window::Window;

mod imp {
    use super::*;

    #[derive(Default)]
    pub(crate) struct Application {
        pub(super) window: OnceCell<WeakRef<Window>>,
        pub(super) provider: gtk::CssProvider,
        pub(super) default_css: OnceCell<String>,
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
    }

    // Sets up keyboard shortcuts
    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<Control>q"]);
    }

    fn setup_css(&self) {
        let imp = self.imp();

        imp.provider
            .load_from_resource("/com/github/marhkb/Pods/style.css");

        imp.default_css.set(imp.provider.to_str().into()).unwrap();

        if let Some(display) = gdk::Display::default() {
            gtk::StyleContext::add_provider_for_display(&display, &imp.provider, 400);
        }
    }

    pub(crate) fn set_headerbar_background(&self, bg_color: Option<gdk::RGBA>) {
        let imp = self.imp();

        let (bg_color, fg_color) = bg_color
            .map(|color| {
                (
                    Cow::Owned(color.to_string()),
                    if luminance(&color) > 0.4 {
                        "rgba(0, 0, 0, 0.8)"
                    } else {
                        "#ffffff"
                    },
                )
            })
            .unwrap_or_else(|| (Cow::Borrowed("@headerbar_bg_color"), "@headerbar_fg_color"));

        self.imp().provider.load_from_data(
            format!(
                "@define-color background_color {};@define-color foreground_color {}; {}",
                bg_color,
                fg_color,
                imp.default_css.get().unwrap()
            )
            .as_bytes(),
        );
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

    pub(crate) fn run(&self) {
        info!("Pods ({})", config::APP_ID);
        info!("Version: {} ({})", config::VERSION, config::PROFILE);
        info!("Datadir: {}", config::PKGDATADIR);

        ApplicationExtManual::run(self);
    }
}

fn srgb(c: f32) -> f32 {
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(color: &gdk::RGBA) -> f32 {
    let red = srgb(color.red());
    let blue = srgb(color.blue());
    let green = srgb(color.green());
    red * 0.2126 + blue * 0.0722 + green * 0.7152
}
