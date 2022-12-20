use gettextrs::gettext;
use glib::subclass::InitializingObject;
use gtk::glib::{self};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/connection/switcher-widget.ui")]
    pub(crate) struct SwitcherWidget {
        pub(super) connection_manager: glib::WeakRef<model::ConnectionManager>,
        #[template_child]
        pub(super) connection_list_view: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SwitcherWidget {
        const NAME: &'static str = "PdsConnectionSwitcherWidget";
        type Type = super::SwitcherWidget;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::bind_template_callbacks(klass);
            klass.set_css_name("connectionswitchermenu");
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl SwitcherWidget {
        #[template_callback]
        fn activated(&self, pos: u32) {
            let connection = self
                .connection_list_view
                .model()
                .unwrap()
                .item(pos)
                .unwrap()
                .downcast::<model::Connection>()
                .unwrap();

            if connection.is_active() {
                return;
            }

            let obj = &*self.obj();
            let connection_manager = obj.connection_manager().unwrap();

            if let Some(widget) = obj.ancestor(gtk::PopoverMenu::static_type()) {
                widget.downcast::<gtk::PopoverMenu>().unwrap().popdown();
            }

            if view::show_ongoing_actions_warning_dialog(
                obj.upcast_ref(),
                &connection_manager,
                &gettext("Confirm Switching Connection"),
            ) {
                obj.switch_connection(&connection_manager, connection.uuid());
            }
        }
    }

    impl ObjectImpl for SwitcherWidget {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::ConnectionManager>(
                    "connection-manager",
                )
                .construct()
                .explicit_notify()
                .build()]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "connection-manager" => self.obj().set_connection_manager(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "connection-manager" => self.obj().connection_manager().to_value(),
                _ => unimplemented!(),
            }
        }

        fn dispose(&self) {
            self.connection_list_view.unparent();
        }
    }

    impl WidgetImpl for SwitcherWidget {}
}

glib::wrapper! {
    pub(crate) struct SwitcherWidget(ObjectSubclass<imp::SwitcherWidget>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for SwitcherWidget {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl SwitcherWidget {
    fn on_error(&self, e: impl ToString) {
        utils::show_error_toast(
            self.upcast_ref(),
            &gettext("Error on switching connection"),
            &e.to_string(),
        );
    }

    pub(crate) fn connection_manager(&self) -> Option<model::ConnectionManager> {
        self.imp().connection_manager.upgrade()
    }

    pub(crate) fn set_connection_manager(&self, value: Option<&model::ConnectionManager>) {
        if self.connection_manager().as_ref() == value {
            return;
        }

        let imp = self.imp();

        if let Some(manager) = value {
            let model = gtk::NoSelection::new(Some(manager));
            imp.connection_list_view.set_model(Some(&model));
        }

        imp.connection_manager.set(value);
        self.notify("connection-manager");
    }

    fn switch_connection(&self, connection_manager: &model::ConnectionManager, uuid: &str) {
        if let Err(e) = connection_manager.set_client_from(uuid) {
            self.on_error(e);
        }
    }
}
