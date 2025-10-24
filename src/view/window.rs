use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::AdwApplicationWindowImpl;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;

use crate::application::Application;
use crate::config;
use crate::model;
use crate::utils;
use crate::view;

const ACTION_CLOSE: &str = "win.close";
const ACTION_GLOBAL_SEARCH: &str = "win.toggle-global-search";
const ACTION_SEARCH: &str = "win.toggle-search";
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
        pub(super) connection_chooser_page: TemplateChild<view::ConnectionChooserPage>,
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
                gdk::Key::F,
                gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
                ACTION_GLOBAL_SEARCH,
            );
            klass.install_action(ACTION_GLOBAL_SEARCH, None, |widget, _, _| {
                widget.toggle_global_search();
            });

            klass.add_binding_action(gdk::Key::F, gdk::ModifierType::CONTROL_MASK, ACTION_SEARCH);
            klass.install_action(ACTION_SEARCH, None, |widget, _, _| {
                widget.toggle_search();
            });

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
                ACTION_CREATE_CONNECTION,
            );
            klass.install_action(ACTION_CREATE_CONNECTION, None, |widget, _, _| {
                widget.add_connection();
            });

            klass.install_action_async(
                ACTION_REMOVE_CONNECTION,
                Some(glib::VariantTy::STRING),
                async |widget, _, data| {
                    let uuid: String = data.unwrap().get().unwrap();
                    widget.remove_connection(&uuid).await;
                },
            );

            klass.add_binding_action(gdk::Key::W, gdk::ModifierType::CONTROL_MASK, ACTION_CLOSE);
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
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::ConnectionManager>(
                        "connection-manager",
                    )
                    .read_only()
                    .build(),
                ]
            })
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

            self.connection_manager.connect_items_changed(clone!(
                #[weak]
                obj,
                move |connection_manager, _, _, _| {
                    if connection_manager.n_items() == 0 {
                        obj.imp().main_stack.set_visible_child_name("welcome");
                    }
                }
            ));

            self.connection_manager.connect_client_notify(clone!(
                #[weak]
                obj,
                move |manager| match manager.client() {
                    Some(client) => client.check_service(
                        clone!(
                            #[weak]
                            obj,
                            move || {
                                obj.imp().main_stack.set_visible_child_full(
                                    "client",
                                    gtk::StackTransitionType::None,
                                );
                            }
                        ),
                        clone!(
                            #[weak]
                            obj,
                            move |e| obj.client_err_op(e)
                        ),
                        clone!(
                            #[weak]
                            obj,
                            #[weak]
                            manager,
                            move |e| {
                                utils::show_error_toast(
                                    &*obj.imp().toast_overlay,
                                    "Connection lost",
                                    &e.to_string(),
                                );
                                manager.unset_client();
                            }
                        ),
                    ),
                    None => {
                        obj.imp().main_stack.set_visible_child_full(
                            if manager.n_items() > 0 {
                                "connection-chooser"
                            } else {
                                "welcome"
                            },
                            gtk::StackTransitionType::Crossfade,
                        );
                    }
                }
            ));

            self.connection_manager.setup(clone!(
                #[weak]
                obj,
                move |result| if let Err(e) = result {
                    obj.on_connection_manager_setup_error(e);
                }
            ));
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
                window,
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
        @implements gtk::Accessible, gio::ActionGroup, gio::ActionMap, gtk::Buildable, gtk::ConstraintTarget,
                    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    pub(crate) fn new(app: &Application) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    pub(crate) fn toggle_global_search(&self) {
        let imp = self.imp();

        match imp
            .main_stack
            .visible_child_name()
            .unwrap_or_default()
            .as_str()
        {
            "client" => imp.client_view.toggle_global_search(),
            "connection-chooser" => imp.connection_chooser_page.toggle_filter_mode(),
            _ => {}
        }
    }

    pub(crate) fn toggle_search(&self) {
        let imp = self.imp();

        match imp
            .main_stack
            .visible_child_name()
            .unwrap_or_default()
            .as_str()
        {
            "client" => imp.client_view.toggle_panel_search(),
            "connection-chooser" => imp.connection_chooser_page.toggle_filter_mode(),
            _ => {}
        }
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
        utils::Dialog::new(
            self,
            &view::ConnectionCreationPage::from(&self.connection_manager()),
        )
        .present();
    }

    pub(crate) async fn remove_connection(&self, uuid: &str) {
        self.connection_manager().remove_connection(uuid).await;
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

        utils::show_error_toast(&*imp.toast_overlay, "Connection lost", &e.to_string());
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
