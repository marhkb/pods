use adw::subclass::prelude::AdwApplicationWindowImpl;
use adw::traits::BinExt;
use cascade::cascade;
use gettextrs::gettext;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::application::Application;
use crate::config;
use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/window.ui")]
    pub(crate) struct Window {
        pub(super) settings: utils::PodsSettings,
        pub(super) connection_manager: model::ConnectionManager,
        #[template_child]
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub(super) search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) title_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) title: TemplateChild<adw::ViewSwitcherTitle>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) search_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) panel_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub(super) images_panel: TemplateChild<view::ImagesPanel>,
        #[template_child]
        pub(super) containers_panel: TemplateChild<view::ContainersPanel>,
        #[template_child]
        pub(super) pods_panel: TemplateChild<view::PodsPanel>,
        #[template_child]
        pub(super) search_panel: TemplateChild<view::SearchPanel>,
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "Window";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            // Initialize all classes here
            view::CircularProgressBar::static_type();
            view::ConnectionChooserPage::static_type();
            view::ConnectionRow::static_type();
            view::ConnectionSwitcherWidget::static_type();
            view::ContainerLogPage::static_type();
            view::ContainerMenuButton::static_type();
            view::ContainerPropertiesGroup::static_type();
            view::ContainerResourcesQuickReferenceGroup::static_type();
            view::ContainersGroup::static_type();
            view::ImageMenuButton::static_type();
            view::ImagePullingPage::static_type();
            view::ImageRowSimple::static_type();
            view::ImageSearchResponseRow::static_type();
            view::ImagesPanel::static_type();
            view::PodMenuButton::static_type();
            view::PodRow::static_type();
            view::PodsPanel::static_type();
            view::PropertyRow::static_type();
            view::PropertyWidgetRow::static_type();
            view::RandomNameEntryRow::static_type();
            view::TextSearchEntry::static_type();
            view::WelcomePage::static_type();
            sourceview5::View::static_type();

            klass.add_binding_action(gdk::Key::F10, gdk::ModifierType::empty(), "menu.show", None);
            klass.install_action("menu.show", None, |widget, _, _| {
                widget.show_menu();
            });

            klass.add_binding_action(
                gdk::Key::Home,
                gdk::ModifierType::ALT_MASK,
                "win.navigate-home",
                None,
            );
            klass.install_action("win.navigate-home", None, |widget, _, _| {
                widget.navigate_home();
            });

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
                "win.add-connection",
                None,
            );
            klass.install_action("win.add-connection", None, |widget, _, _| {
                widget.add_connection();
            });

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                "entity.create",
                None,
            );
            klass.install_action("entity.create", None, move |widget, _, _| {
                widget.create_entity();
            });

            klass.install_action("win.remove-connection", Some("s"), |widget, _, data| {
                let uuid: String = data.unwrap().get().unwrap();
                widget.remove_connection(&uuid);
            });

            klass.install_action("win.show-podman-info", None, |widget, _, _| {
                widget.show_podman_info_dialog();
            });

            klass.add_binding_action(
                gdk::Key::F,
                gdk::ModifierType::CONTROL_MASK,
                "win.toggle-search",
                None,
            );

            klass.install_action("win.toggle-search", None, |widget, _, _| {
                widget.toggle_search();
            });
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "connection-manager",
                    "Connection Manager",
                    "The connection manager client",
                    model::ConnectionManager::static_type(),
                    glib::ParamFlags::READABLE | glib::ParamFlags::EXPLICIT_NOTIFY,
                )]
            });

            PROPERTIES.as_ref()
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "connection-manager" => obj.connection_manager().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            // Devel Profile
            if config::PROFILE == "Devel" {
                obj.add_css_class("devel");
            }

            // Load settings.
            obj.load_settings();

            let popover_menu = self
                .menu_button
                .popover()
                .unwrap()
                .downcast::<gtk::PopoverMenu>()
                .unwrap();

            popover_menu.set_widget_name("main-menu");

            popover_menu.add_child(
                &view::ConnectionSwitcherWidget::from(&self.connection_manager),
                "connections",
            );

            self.search_button
                .connect_active_notify(clone!(@weak obj => move |button| {
                    let imp = obj.imp();

                    if button.is_active() {
                        imp.title_stack.set_visible_child(&*imp.search_entry);
                        imp.search_entry.grab_focus();
                        imp.search_stack.set_visible_child(&*imp.search_panel);
                    } else {
                        imp.search_entry.set_text("");
                        imp.title_stack.set_visible_child(&*imp.title);
                        imp.search_stack.set_visible_child_name("main");
                    }
                }));

            self.search_entry
                .connect_text_notify(clone!(@weak obj => move |entry| {
                    let imp = obj.imp();

                    imp.search_panel.set_term(entry.text().into());
                    if !entry.text().is_empty() {
                        imp.search_button.set_active(true);
                    }
                }));

            let search_entry_key_ctrl = gtk::EventControllerKey::new();
            search_entry_key_ctrl.connect_key_pressed(
                clone!(@weak obj => @default-return gtk::Inhibit(false), move |_, key, _, _| {
                    if key == gdk::Key::Escape {
                        obj.imp().search_button.set_active(false);
                        gtk::Inhibit(true)
                    } else {
                        gtk::Inhibit(false)
                    }
                }),
            );
            self.search_entry.add_controller(&search_entry_key_ctrl);

            self.search_entry.set_key_capture_widget(Some(obj));
            self.leaflet
                .connect_visible_child_notify(clone!(@weak obj => move |leaflet| {
                    obj.imp().search_entry.set_key_capture_widget(
                        if leaflet.visible_child_name().as_deref() == Some("overlay") {
                            None
                        } else {
                            Some(&obj)
                        });
                }));

            gtk::Stack::this_expression("visible-child-name")
                .chain_closure::<bool>(closure!(|_: gtk::Stack, visible_child_name: &str| {
                    visible_child_name == "title"
                }))
                .bind(&*self.menu_button, "visible", Some(&*self.title_stack));

            self.connection_manager.connect_notify_local(
                Some("client"),
                clone!(@weak obj => move |manager, _| match manager.client() {
                    Some(client) => client.check_service(
                        clone!(@weak obj, @weak client => move || {
                            let imp = obj.imp();
                            imp.search_button.set_active(false);
                            imp.main_stack.set_visible_child_full("client", gtk::StackTransitionType::None);
                            obj.set_headerbar_background(client.connection().rgb());
                        }),
                        clone!(@weak obj => move |e| obj.client_err_op(e)),
                        clone!(@weak obj, @weak manager => move |e| {
                            utils::show_error_toast(&obj, "Connection lost", &e.to_string());
                            manager.unset_client();
                        }),
                    ),
                    None => {
                        let imp = obj.imp();

                        imp.leaflet_overlay.hide_details();
                        imp.main_stack.set_visible_child_full(
                            if manager.n_items() > 0 {
                                "connection-chooser"
                            } else {
                                "welcome"
                            },
                            gtk::StackTransitionType::Crossfade
                        );

                        obj.set_headerbar_background(None);
                    }
                }),
            );

            match self.connection_manager.setup() {
                Ok(_) => {
                    if self.connection_manager.n_items() == 0 {
                        self.main_stack.set_visible_child_name("welcome");
                    }
                }
                Err(e) => obj.on_connection_manager_setup_error(e),
            }
        }
    }

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {
        // Save window state on delete event
        fn close_request(&self, window: &Self::Type) -> gtk::Inhibit {
            if let Err(err) = window.save_window_size() {
                log::warn!("Failed to save window state, {}", &err);
            }

            // Pass close request on to the parent
            self.parent_close_request(window)
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
        glib::Object::new(&[("application", app)]).expect("Failed to create Window")
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let (width, height) = self.default_size();

        let imp = self.imp();
        imp.settings.set_int("window-width", width)?;
        imp.settings.set_int("window-height", height)?;
        imp.settings
            .set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_settings(&self) {
        let settings = &*self.imp().settings;

        let width = settings.int("window-width");
        let height = settings.int("window-height");
        let is_maximized = settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }

        settings
            .bind(
                "last-used-view",
                &*self.imp().panel_stack,
                "visible-child-name",
            )
            .build();
    }

    pub(crate) fn connection_manager(&self) -> model::ConnectionManager {
        self.imp().connection_manager.clone()
    }

    fn set_headerbar_background(&self, rgb: Option<gdk::RGBA>) {
        self.application()
            .unwrap()
            .downcast::<crate::Application>()
            .unwrap()
            .set_headerbar_background(rgb);
    }

    fn on_connection_manager_setup_error(&self, e: impl ToString) {
        let imp = self.imp();
        imp.main_stack
            .set_visible_child_name(if imp.connection_manager.n_items() > 0 {
                "connection-chooser"
            } else {
                "welcome"
            });

        utils::show_error_toast(self, "Connection error", &e.to_string());
    }

    fn show_menu(&self) {
        let imp = self.imp();
        if imp.leaflet_overlay.child().is_none() {
            imp.menu_button.popup();
        }
    }

    fn navigate_home(&self) {
        self.leaflet_overlay().hide_details();
    }

    fn add_connection(&self) {
        let leaflet_overlay = &*self.imp().leaflet_overlay;

        if leaflet_overlay.child().is_none() {
            leaflet_overlay.show_details(&view::ConnectionCreatorPage::from(
                &self.connection_manager(),
            ));
        }
    }

    fn create_entity(&self) {
        let imp = self.imp();
        let leaflet_overlay = &*imp.leaflet_overlay;

        if self.connection_manager().client().is_some() && leaflet_overlay.child().is_none() {
            imp.panel_stack
                .visible_child_name()
                .map(|name| match name.as_str() {
                    "images" => imp.images_panel.activate_action("images.pull", None),
                    "containers" => imp
                        .containers_panel
                        .activate_action("containers.create", None),
                    "pods" => imp.pods_panel.activate_action("pods.create", None),
                    _ => unreachable!(),
                });
        }
    }

    fn remove_connection(&self, uuid: &str) {
        self.connection_manager().remove_connection(uuid);
    }

    fn show_podman_info_dialog(&self) {
        cascade! {
            view::InfoDialog::from(self.connection_manager().client().as_ref());
            ..set_transient_for(Some(self));
        }
        .present();
    }

    fn toggle_search(&self) {
        let imp = self.imp();
        imp.search_button.set_active(!imp.search_button.is_active());
    }

    fn client_err_op(&self, e: model::ClientError) {
        self.show_toast(
            &adw::Toast::builder()
                .title(&match e.err {
                    model::RefreshError::List => gettext!(
                        "Error on loading {}",
                        match e.variant {
                            model::ClientErrorVariant::Images => gettext("images"),
                            model::ClientErrorVariant::Containers => gettext("containers"),
                            model::ClientErrorVariant::Pods => gettext("pods"),
                        }
                    ),
                    model::RefreshError::Inspect(id) => {
                        // Translators: "{}" is the placeholder for the image id.
                        gettext!(
                            "Error on inspecting {} '{}'",
                            match e.variant {
                                model::ClientErrorVariant::Images => gettext("image"),
                                model::ClientErrorVariant::Containers => gettext("container"),
                                model::ClientErrorVariant::Pods => gettext("pods"),
                            },
                            id
                        )
                    }
                })
                .timeout(3)
                .priority(adw::ToastPriority::High)
                .build(),
        );
    }

    pub(crate) fn show_toast(&self, toast: &adw::Toast) {
        self.imp().toast_overlay.add_toast(toast);
    }

    pub(crate) fn leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::leaflet_overlay(&*self.imp().leaflet)
    }
}
