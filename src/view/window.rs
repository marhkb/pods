use adw::prelude::*;
use adw::subclass::prelude::AdwApplicationWindowImpl;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::once_cell::sync::Lazy;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::CompositeTemplate;

use crate::application::Application;
use crate::config;
use crate::model;
use crate::utils;
use crate::view;

const ACTION_CLOSE: &str = "win.close";
const ACTION_CREATE_CONNECTION: &str = "win.create-connection";
const ACTION_REMOVE_CONNECTION: &str = "win.remove-connection";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/window.ui")]
    pub(crate) struct Window {
        pub(super) settings: utils::PodsSettings,
        pub(super) connection_manager: model::ConnectionManager,
        #[template_child]
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) client_view: TemplateChild<view::ClientView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "PdsWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
                ACTION_CREATE_CONNECTION,
                None,
            );
            klass.install_action(ACTION_CREATE_CONNECTION, None, |widget, _, _| {
                widget.add_connection();
            });

            klass.install_action(ACTION_REMOVE_CONNECTION, Some("s"), |widget, _, data| {
                let uuid: String = data.unwrap().get().unwrap();
                widget.remove_connection(&uuid);
            });

            klass.add_binding_action(
                gdk::Key::W,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_CLOSE,
                None,
            );
            klass.install_action(ACTION_CLOSE, None, |widget, _, _| {
                widget.close();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::ConnectionManager>(
                    "connection-manager",
                )
                .read_only()
                .build()]
            });

            PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "connection-manager" => self.obj().connection_manager().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let shortcuts: gtk::ShortcutsWindow =
                gtk::Builder::from_resource("/com/github/marhkb/Pods/ui/view/shortcuts.ui")
                    .object("shortcuts")
                    .unwrap();

            let controller = gtk::EventControllerKey::new();
            controller.connect_key_pressed(clone!(
                @weak shortcuts => @default-return glib::Propagation::Stop, move |_, key, _, modifier| {
                    if key == gdk::Key::w && modifier == gdk::ModifierType::CONTROL_MASK {
                        shortcuts.close();
                    }
                    glib::Propagation::Proceed
                }
            ));
            shortcuts.add_controller(controller);

            obj.set_help_overlay(Some(&shortcuts));

            if config::PROFILE == "Devel" {
                obj.add_css_class("devel");
            }

            let width = self.settings.int("window-width");
            let height = self.settings.int("window-height");
            let is_maximized = self.settings.boolean("is-maximized");
            obj.set_default_size(width, height);

            if is_maximized {
                obj.maximize();
            }

            let style_manager = adw::StyleManager::default();

            self.settings
                .bind("color-scheme", &style_manager, "color-scheme")
                .get()
                .set_mapping(|value, _| {
                    Some(color_scheme_to_str(value.get().unwrap()).to_variant())
                })
                .set()
                .mapping(|variant, _| Some(str_to_color_scheme(variant.str().unwrap()).to_value()))
                .build();

            let action = gio::SimpleAction::new_stateful(
                "theme",
                Some(glib::VariantTy::STRING),
                &color_scheme_to_str(style_manager.color_scheme()).to_variant(),
            );
            action.connect_activate(clone!(@weak self as obj => move |_, param| {
                adw::StyleManager::default()
                    .set_color_scheme(str_to_color_scheme(param.unwrap().str().unwrap()));
            }));
            obj.add_action(&action);

            adw::StyleManager::default().connect_color_scheme_notify(
                clone!(@weak action => move |style_manager| {
                    action.set_state(&color_scheme_to_str(style_manager.color_scheme()).to_variant());
                }),
            );

            self.connection_manager.connect_client_notify(
                clone!(@weak obj => move |manager| match manager.client() {
                    Some(client) => client.check_service(
                        clone!(@weak obj, @weak client => move || {
                            obj
                                .imp()
                                .main_stack
                                .set_visible_child_full("client", gtk::StackTransitionType::None);
                        }),
                        clone!(@weak obj => move |e| obj.client_err_op(e)),
                        clone!(@weak obj, @weak manager => move |e| {
                            utils::show_error_toast(
                                obj.imp().toast_overlay.upcast_ref(),
                                "Connection lost",
                                &e.to_string()
                            );
                            manager.unset_client();
                        }),
                    ),
                    None => {
                        obj.imp().main_stack.set_visible_child_full(
                            if manager.n_items() > 0 {
                                "connection-chooser"
                            } else {
                                "welcome"
                            },
                            gtk::StackTransitionType::Crossfade
                        );
                    }
                }),
            );

            self.connection_manager
                .setup(clone!(@weak obj => move |result| match result {
                    Ok(_) => if obj.connection_manager().n_items() == 0 {
                        obj.imp().main_stack.set_visible_child_name("welcome");
                    }
                    Err(e) => obj.on_connection_manager_setup_error(e),
                }));
        }
    }

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {
        // Save window state on delete event
        fn close_request(&self) -> glib::Propagation {
            let window = &*self.obj();

            if let Err(err) = window.save_window_size() {
                log::warn!("Failed to save window state, {}", &err);
            }

            if view::show_ongoing_actions_warning_dialog(
                window.upcast_ref(),
                &self.connection_manager,
                &gettext("Confirm Exiting The Application"),
            ) {
                self.parent_close_request()
            } else {
                glib::Propagation::Stop
            }
        }
    }

    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub(crate) struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Root;
}

impl Window {
    pub(crate) fn new(app: &Application) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    pub(crate) fn connection_manager(&self) -> model::ConnectionManager {
        self.imp().connection_manager.clone()
    }

    pub(crate) fn toast_overlay(&self) -> adw::ToastOverlay {
        self.imp().toast_overlay.get()
    }

    pub(crate) fn navigation_view(&self) -> &adw::NavigationView {
        self.imp().client_view.navigation_view()
    }

    pub(crate) fn add_connection(&self) {
        utils::show_dialog(
            self.upcast_ref(),
            view::ConnectionCreationPage::from(&self.connection_manager()).upcast_ref(),
        );
    }

    pub(crate) fn remove_connection(&self, uuid: &str) {
        self.connection_manager().remove_connection(uuid);
    }

    pub(crate) fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let (width, height) = self.default_size();

        let imp = self.imp();
        imp.settings.set_int("window-width", width)?;
        imp.settings.set_int("window-height", height)?;
        imp.settings
            .set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn on_connection_manager_setup_error(&self, e: impl ToString) {
        let imp = self.imp();
        imp.main_stack
            .set_visible_child_name(if imp.connection_manager.n_items() > 0 {
                "connection-chooser"
            } else {
                "welcome"
            });

        utils::show_error_toast(
            imp.toast_overlay.upcast_ref(),
            "Connection lost",
            &e.to_string(),
        );
    }

    fn client_err_op(&self, e: model::ClientError) {
        self.imp().toast_overlay.add_toast(
            adw::Toast::builder()
                .title(match e {
                    model::ClientError::Images => gettext("Error on loading images"),
                    model::ClientError::Containers => gettext("Error on loading containers"),
                    model::ClientError::Pods => gettext("Error on loading pods"),
                    model::ClientError::Volumes => gettext("Error on loading volumes"),
                })
                .timeout(3)
                .priority(adw::ToastPriority::High)
                .build(),
        );
    }
}

fn str_to_color_scheme(scheme: &str) -> adw::ColorScheme {
    match scheme {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    }
}

fn color_scheme_to_str(scheme: adw::ColorScheme) -> &'static str {
    match scheme {
        adw::ColorScheme::ForceDark | adw::ColorScheme::PreferDark => "dark",
        adw::ColorScheme::ForceLight | adw::ColorScheme::PreferLight => "light",
        _ => "default",
    }
}
