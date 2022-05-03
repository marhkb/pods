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

use crate::api;
use crate::application::Application;
use crate::config;
use crate::model;
use crate::utils;
use crate::view;
use crate::PODMAN;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/window.ui")]
    pub(crate) struct Window {
        pub(super) settings: utils::PodsSettings,
        pub(super) client: model::Client,
        #[template_child]
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) start_service_page: TemplateChild<view::StartServicePage>,
        #[template_child]
        pub(super) leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub(super) images_search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) containers_search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) panel_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub(super) images_panel: TemplateChild<view::ImagesPanel>,
        #[template_child]
        pub(super) containers_panel: TemplateChild<view::ContainersPanel>,
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
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
            view::CircularProgressBar::static_type();
            view::ContainerDetailsPanel::static_type();
            view::ContainerLogsPanel::static_type();
            view::ContainersGroup::static_type();
            view::ImageRowSimple::static_type();
            view::ImageSearchResponseRow::static_type();
            view::ImagesPanel::static_type();
            view::PropertyWidgetRow::static_type();
            view::StartServicePage::static_type();
            view::TextSearchEntry::static_type();
            sourceview5::View::static_type();

            klass.install_action("win.show-podman-info", None, |widget, _, _| {
                widget.show_podman_info_dialog();
            });

            klass.install_action("image.pull", None, move |widget, _, _| {
                widget.show_pull_dialog();
            });
            klass.install_action("images.prune-unused", None, move |widget, _, _| {
                widget.show_prune_page();
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
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            // Devel Profile
            if config::PROFILE == "Devel" {
                obj.add_css_class("devel");
            }

            // Load settings.
            obj.load_settings();

            self.menu_button
                .popover()
                .unwrap()
                .downcast::<gtk::PopoverMenu>()
                .unwrap()
                .add_child(&view::ThemeSelector::default(), "theme");

            self.images_panel.set_image_list(self.client.image_list());
            self.containers_panel
                .set_container_list(self.client.container_list());

            self.images_panel
                .connect_search_button(&*self.images_search_button);

            self.containers_panel
                .connect_search_button(&*self.containers_search_button);

            let visible_child_name_expr = adw::ViewStack::this_expression("visible-child-name");

            visible_child_name_expr
                .chain_closure::<bool>(closure!(
                    |_: glib::Object, name: Option<&str>| name == Some("images")
                ))
                .bind(
                    &*self.images_search_button,
                    "visible",
                    Some(&*self.panel_stack),
                );
            visible_child_name_expr
                .chain_closure::<bool>(closure!(
                    |_: glib::Object, name: Option<&str>| name == Some("containers")
                ))
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

    fn show_podman_info_dialog(&self) {
        cascade! {
            view::InfoDialog::default();
            ..set_transient_for(Some(self));
        }
        .present();
    }

    fn show_pull_dialog(&self) {
        self.imp()
            .leaflet_overlay
            .show_details(&view::ImagePullPage::from(&self.imp().client));
    }

    fn show_prune_page(&self) {
        self.imp().images_panel.show_prune_page();
    }

    fn toggle_search(&self) {
        let imp = self.imp();

        if imp.leaflet_overlay.child().is_none() {
            match imp.panel_stack.visible_child_name().as_deref() {
                Some("images") => imp.images_panel.toggle_search(),
                Some("containers") => imp.containers_panel.toggle_search(),
                _ => unreachable!(),
            }
        }
    }

    pub(crate) fn check_service(&self) {
        let imp = self.imp();

        // We disable the start service page here in order to prevent the button from flashing to
        // `sensitive` at the beginning of the transition to the main view.
        imp.start_service_page.set_enabled(false);
        // Same reason applies here as above.
        imp.connection_lost_page.set_enabled(false);

        utils::do_async(
            PODMAN.ping(),
            clone!(@weak self as obj => move |result| {
                let imp = obj.imp();
                match result {
                    Ok(_) => {
                        imp.main_stack.set_visible_child(&*imp.leaflet);
                        imp.images_panel.image_list().unwrap().refresh(
                            clone!(@weak obj => move |e| {
                                obj.images_err_op(e);
                            }),
                        );
                        imp.containers_panel
                            .container_list()
                            .unwrap()
                            .refresh(clone!(@weak obj => move |e| {
                                obj.containers_err_op(e);
                            }));

                        obj.start_event_listener();
                    }
                    Err(e) => {
                        imp.start_service_page.set_enabled(true);
                        imp.main_stack.set_visible_child(&*imp.start_service_page);
                        log::error!("Could not connect to Podman: {e}");
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
                            "image" => imp.images_panel.image_list().unwrap().handle_event(
                                event,
                                clone!(@weak obj => move |e| obj.images_err_op(e)),
                            ),
                            "container" => imp
                                .containers_panel
                                .container_list()
                                .unwrap()
                                .handle_event(
                                    event,
                                    clone!(@weak obj => move |e| obj.containers_err_op(e)),
                                ),
                            other => log::warn!("Unhandled event type: {other}"),
                        }
                        true
                    },
                    Err(e) => {
                        log::error!("Stopping image event stream due to error: {e}");

                        imp.connection_lost_page.set_enabled(true);
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
                        // Translators: "{}" is the placeholder for the image id.
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
                        // Translators: "{}" is the placeholder for the container id.
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

    pub(crate) fn leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::leaflet_overlay(&*self.imp().leaflet)
    }
}
