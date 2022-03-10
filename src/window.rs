use std::time::Duration;

use adw::subclass::prelude::AdwApplicationWindowImpl;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, CompositeTemplate};

use crate::application::Application;
use crate::config::{APP_ID, PROFILE};
use crate::{utils, view, PODMAN};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/window.ui")]
    pub(crate) struct Window {
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) start_service_page: TemplateChild<view::StartServicePage>,
        #[template_child]
        pub(super) leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub(super) sidebar: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) panel_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) images_panel: TemplateChild<view::ImagesPanel>,
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
            klass.install_action("leaflet.back", None, move |widget, _, _| {
                widget
                    .imp()
                    .leaflet
                    .navigate(adw::NavigationDirection::Back);
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
            if PROFILE == "Devel" {
                obj.add_css_class("devel");
            }

            // Load latest window state
            obj.load_window_size();

            self.list_box.bind_model(
                Some(&gtk::SingleSelection::new(Some(&self.panel_stack.pages()))),
                |obj| {
                    let stack_page = obj.downcast_ref::<gtk::StackPage>().unwrap();

                    view::SidebarRow::new(
                        stack_page.icon_name().as_deref(),
                        stack_page.name().unwrap().as_str(),
                        stack_page.title().as_deref(),
                    )
                    .upcast()
                },
            );

            obj.setup_navigation();

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

        let settings = gio::Settings::new(APP_ID);

        settings.set_int("window-width", width)?;
        settings.set_int("window-height", height)?;
        settings.set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let settings = gio::Settings::new(APP_ID);

        let width = settings.int("window-width");
        let height = settings.int("window-height");
        let is_maximized = settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }

    fn setup_navigation(&self) {
        let imp = self.imp();

        self.action_set_enabled("leaflet.back", imp.leaflet.is_folded());

        self.set_selected_sidebar_row();
        imp.leaflet
            .connect_folded_notify(clone!(@weak self as obj => move |_| {
                obj.set_selected_sidebar_row();
            }));

        imp.list_box
            .connect_selected_rows_changed(clone!(@weak self as obj => move |list_box| {
                if let Some(row) = list_box.selected_row() {
                    let imp = obj.imp();

                    imp.leaflet.navigate(adw::NavigationDirection::Forward);
                    imp.panel_stack.set_visible_child_name(
                        row
                            .child()
                            .unwrap()
                            .downcast_ref::<view::SidebarRow>()
                            .unwrap()
                            .panel_name()
                    );
                }
            }));
    }

    fn set_selected_sidebar_row(&self) {
        let imp = self.imp();

        self.action_set_enabled("leaflet.back", imp.leaflet.is_folded());

        if imp.leaflet.is_folded() {
            imp.list_box.unselect_all();
        } else {
            imp.list_box.select_row(
                imp.list_box
                    .row_at_index(imp.panel_stack.pages().selection().minimum() as i32)
                    .as_ref(),
            );
        }
    }

    pub(crate) fn check_service(&self) {
        utils::do_async(
            PODMAN.ping(),
            clone!(@weak self as obj => move |result| match result {
                Ok(_) => {
                    let imp = obj.imp();
                    imp.main_stack.set_visible_child(&*imp.leaflet);
                    imp.images_panel.image_list().setup();

                    obj.periodic_service_check();
                }
                Err(e) => {
                    log::error!("Could not connect to podman: {e}");
                    // TODO: Show a toast message
                }
            }),
        );
    }

    fn periodic_service_check(&self) {
        utils::do_async(
            async {
                while PODMAN.ping().await.is_ok() {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            },
            clone!(@weak self as obj => move |_| {
                let imp = obj.imp();
                imp.main_stack.set_visible_child(&*imp.connection_lost_page);
            }),
        );
    }
}
