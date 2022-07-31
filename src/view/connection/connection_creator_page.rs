use adw::traits::ActionRowExt;
use gettextrs::gettext;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::model;
use crate::utils;
use crate::view;
use crate::window::Window;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/connection-creator-page.ui")]
    pub(crate) struct ConnectionCreatorPage {
        pub(super) connection_manager: OnceCell<model::ConnectionManager>,
        #[template_child]
        pub(super) name_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) unix_socket_url_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) custom_url_radio_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) url_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) color_button: TemplateChild<gtk::ColorButton>,
        #[template_child]
        pub(super) color_switch: TemplateChild<gtk::Switch>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConnectionCreatorPage {
        const NAME: &'static str = "ConnectionCreatorPage";
        type Type = super::ConnectionCreatorPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("navigation.go-first", None, move |widget, _, _| {
                widget.navigate_to_first();
            });
            klass.install_action("navigation.back", None, move |widget, _, _| {
                widget.navigate_back();
            });

            klass.install_action("client.try-connect", None, move |widget, _, _| {
                widget.try_connect();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ConnectionCreatorPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "connection-manager",
                    "Connection Manager",
                    "The connection manager client",
                    model::ConnectionManager::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                )]
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
                "connection-manager" => self.connection_manager.set(value.get().unwrap()).unwrap(),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "connection-manager" => obj.connection_manager().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.action_set_enabled("client.try-connect", !self.name_entry_row.text().is_empty());
            self.name_entry_row
                .connect_changed(clone!(@weak obj => move |entry| {
                    obj.action_set_enabled("client.try-connect", !entry.text().is_empty())
                }));

            self.unix_socket_url_row
                .set_subtitle(&utils::unix_socket_url());

            self.custom_url_radio_button
                .set_active(obj.connection_manager().contains_local_connection());

            self.color_button
                .set_rgba(&gdk::RGBA::new(0.207, 0.517, 0.894, 1.0));
        }

        fn dispose(&self, obj: &Self::Type) {
            let mut child = obj.first_child();
            while let Some(child_) = child {
                child = child_.next_sibling();
                child_.unparent();
            }
        }
    }

    impl WidgetImpl for ConnectionCreatorPage {
        fn realize(&self, widget: &Self::Type) {
            self.parent_realize(widget);

            widget.action_set_enabled(
                "navigation.go-first",
                widget.previous_leaflet_overlay() != widget.root_leaflet_overlay(),
            );

            glib::idle_add_local(
                clone!(@weak widget => @default-return glib::Continue(false), move || {
                    widget.imp().name_entry_row.grab_focus();
                    glib::Continue(false)
                }),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct ConnectionCreatorPage(ObjectSubclass<imp::ConnectionCreatorPage>)
        @extends gtk::Widget;
}

impl From<&model::ConnectionManager> for ConnectionCreatorPage {
    fn from(connection_manager: &model::ConnectionManager) -> Self {
        glib::Object::new(&[("connection-manager", connection_manager)])
            .expect("Failed to create ConnectionCreatorPage")
    }
}

impl ConnectionCreatorPage {
    pub(crate) fn connection_manager(&self) -> &model::ConnectionManager {
        self.imp().connection_manager.get().unwrap()
    }

    fn try_connect(&self) {
        let imp = self.imp();

        if let Err(e) = self.connection_manager().try_connect(
            imp.name_entry_row.text().as_str(),
            if imp.custom_url_radio_button.is_active() {
                imp.url_entry_row.text().into()
            } else {
                utils::unix_socket_url()
            }
            .as_ref(),
            if imp.color_switch.is_active() {
                Some(imp.color_button.rgba())
            } else {
                None
            },
            clone!(@weak self as obj => move |result| match result {
                Ok(_) => obj.navigate_to_first(),
                Err(e) => obj.on_error(e),
            }),
        ) {
            self.on_error(e);
        }
    }

    fn on_error(&self, e: impl ToString) {
        utils::show_error_toast(self, &gettext("Error"), &e.to_string());
    }

    fn navigate_to_first(&self) {
        self.root_leaflet_overlay().hide_details();
    }

    fn navigate_back(&self) {
        self.previous_leaflet_overlay().hide_details();
    }

    fn previous_leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::find_parent_leaflet_overlay(self)
    }

    fn root_leaflet_overlay(&self) -> view::LeafletOverlay {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .leaflet_overlay()
    }
}
