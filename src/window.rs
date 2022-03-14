use adw::subclass::prelude::AdwApplicationWindowImpl;
use gettextrs::gettext;
use gtk::glib::{clone, closure};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib, CompositeTemplate};
use once_cell::sync::Lazy;

use crate::application::Application;
use crate::{api, config, model, utils, view, PODMAN};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/window.ui")]
    pub(crate) struct Window {
        #[template_child]
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) start_service_page: TemplateChild<view::StartServicePage>,
        #[template_child]
        pub(super) main_view_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) images_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) images_search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) containers_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) containers_search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) panel_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub(super) images_panel: TemplateChild<view::ImagesPanel>,
        #[template_child]
        pub(super) containers_panel: TemplateChild<view::ContainersPanel>,
        #[template_child]
        pub(super) connection_lost_page: TemplateChild<view::ConnectionLostPage>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "Window";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            // Initialize all classes here
            view::CheckServicePage::static_type();
            view::ImageRowSimple::static_type();
            view::ImagesPanel::static_type();
            view::StartServicePage::static_type();

            klass.install_property_action("images.show-intermediates", "show-intermediate-images");
            klass.install_action("images.prune-unused", None, move |widget, _, _| {
                widget.imp().images_panel.show_prune_dialog();
            });

            klass.install_property_action(
                "containers.show-only-running",
                "show-only-running-containers",
            );

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
                vec![
                    glib::ParamSpecBoolean::new(
                        "show-intermediate-images",
                        "Show Intermediate Images",
                        "Whether to also show intermediate images",
                        bool::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecBoolean::new(
                        "show-only-running-containers",
                        "Show Only Running Containers",
                        "Whether to show only running containers",
                        true,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "show-intermediate-images" => {
                    self.images_panel
                        .set_show_intermediates(value.get().unwrap());
                }
                "show-only-running-containers" => {
                    self.containers_panel
                        .set_show_only_running(value.get().unwrap());
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "show-intermediate-images" => self
                    .images_panel
                    .try_get()
                    .as_ref()
                    .map(view::ImagesPanel::show_intermediates)
                    .unwrap_or_default()
                    .to_value(),
                "show-only-running-containers" => self
                    .containers_panel
                    .try_get()
                    .as_ref()
                    .map(view::ContainersPanel::show_only_running)
                    .unwrap_or(true)
                    .to_value(),
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

            self.images_panel
                .connect_search_button(&*self.images_search_button);

            self.containers_panel
                .connect_search_button(&*self.containers_search_button);

            obj.notify("show-intermediate-images");
            self.images_panel.connect_notify_local(
                Some("show-intermediates"),
                clone!(@weak obj => move |_, _| obj.notify("show-intermediate-images")),
            );

            obj.notify("show-only-running-containers");
            self.containers_panel.connect_notify_local(
                Some("show-only-running"),
                clone!(@weak obj => move |_, _| obj.notify("show-only-running-containers")),
            );

            adw::ViewStack::this_expression("visible-child-name")
                .chain_closure::<bool>(closure!(|_: glib::Object, name: Option<&str>| {
                    name == Some("images")
                }))
                .bind(
                    &*self.images_menu_button,
                    "visible",
                    Some(&*self.panel_stack),
                );
            adw::ViewStack::this_expression("visible-child-name")
                .chain_closure::<bool>(closure!(|_: glib::Object, name: Option<&str>| {
                    name == Some("images")
                }))
                .bind(
                    &*self.images_search_button,
                    "visible",
                    Some(&*self.panel_stack),
                );

            adw::ViewStack::this_expression("visible-child-name")
                .chain_closure::<bool>(closure!(|_: glib::Object, name: Option<&str>| {
                    name == Some("containers")
                }))
                .bind(
                    &*self.containers_menu_button,
                    "visible",
                    Some(&*self.panel_stack),
                );
            adw::ViewStack::this_expression("visible-child-name")
                .chain_closure::<bool>(closure!(|_: glib::Object, name: Option<&str>| {
                    name == Some("containers")
                }))
                .bind(
                    &*self.containers_search_button,
                    "visible",
                    Some(&*self.panel_stack),
                );

            obj.check_service();
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

        let settings = gio::Settings::new(config::APP_ID);

        settings.set_int("window-width", width)?;
        settings.set_int("window-height", height)?;
        settings.set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_settings(&self) {
        let settings = gio::Settings::new(config::APP_ID);

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

    fn toggle_search(&self) {
        let imp = self.imp();

        match imp.panel_stack.visible_child_name().as_deref() {
            Some("images") => imp.images_panel.toggle_search(),
            Some("containers") => imp.containers_panel.toggle_search(),
            _ => unreachable!(),
        }
    }

    pub(crate) fn check_service(&self) {
        utils::do_async(
            PODMAN.ping(),
            clone!(@weak self as obj => move |result| {
                let imp = obj.imp();
                match result {
                    Ok(_) => {
                        imp.main_stack.set_visible_child(&*imp.main_view_box);
                        imp.images_panel.image_list().refresh(clone!(@weak obj => move |e| {
                            obj.images_err_op(e);
                        }));
                        imp.containers_panel.container_list().refresh(clone!(@weak obj => move |e| {
                            obj.containers_err_op(e);
                        }));

                        obj.start_event_listener();
                    }
                    Err(e) => {
                        imp.main_stack.set_visible_child(&*imp.start_service_page);
                        log::error!("Could not connect to podman: {e}");
                        // No need to show a toast. The start service page is enough.
                    }
                }
            }),
        );
    }

    fn start_event_listener(&self) {
        utils::run_stream(
            PODMAN.events(&api::EventsOpts::builder().build()),
            clone!(
                @weak self as obj => @default-return glib::Continue(false),
                move |result|
            {
                let imp = obj.imp();

                glib::Continue(match result {
                    Ok(event) => {
                        log::debug!("Event: {event:?}");
                        match event.typ.as_str() {
                            "image" => imp.images_panel.image_list().handle_event(
                                event,
                                clone!(@weak obj => move |e| obj.images_err_op(e)),
                            ),
                            "container" => imp.containers_panel.container_list().handle_event(
                                event,
                                clone!(@weak obj => move |e| obj.containers_err_op(e)),
                            ),
                            other => log::warn!("Unhandled event type: {other}"),
                        }
                        true
                    },
                    Err(e) => {
                        log::error!("Stopping image event stream due to error: {e}");

                        imp.main_stack.set_visible_child(&*imp.connection_lost_page);
                        false
                    }
                })
            }),
        );
    }

    fn images_err_op(&self, e: model::ImageListError) {
        self.show_toast(
            &adw::Toast::builder()
                .title(&match e {
                    model::ImageListError::List => gettext("Error on loading images"),
                    model::ImageListError::Inspect(id) => {
                        gettext!("Error on inspecting image '{}'", id)
                    }
                })
                .timeout(3)
                .priority(adw::ToastPriority::High)
                .build(),
        );
    }

    fn containers_err_op(&self, e: model::ContainerListError) {
        self.show_toast(
            &adw::Toast::builder()
                .title(&match e {
                    model::ContainerListError::List => gettext("Error on loading containers"),
                    model::ContainerListError::Inspect(id) => {
                        gettext!("Error on inspecting container '{}'", id)
                    }
                })
                .timeout(3)
                .priority(adw::ToastPriority::High)
                .build(),
        )
    }

    pub(crate) fn show_toast(&self, toast: &adw::Toast) {
        self.imp().toast_overlay.add_toast(toast);
    }
}
