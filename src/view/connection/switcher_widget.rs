use gettextrs::gettext;
use glib::clone;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::SwitcherWidget)]
    #[template(resource = "/com/github/marhkb/Pods/ui/connection/switcher-widget.ui")]
    pub(crate) struct SwitcherWidget {
        #[property(get, set = Self::set_connection_manager, construct, explicit_notify, nullable)]
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
            klass.bind_template();
            klass.bind_template_callbacks();
            klass.set_css_name("connectionswitchermenu");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
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
                obj.switch_connection(&connection_manager, &connection.uuid());
            }
        }
    }

    impl ObjectImpl for SwitcherWidget {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn dispose(&self) {
            self.connection_list_view.unparent();
        }
    }

    impl WidgetImpl for SwitcherWidget {}

    impl SwitcherWidget {
        pub(super) fn set_connection_manager(&self, value: Option<&model::ConnectionManager>) {
            let obj = &*self.obj();
            if obj.connection_manager().as_ref() == value {
                return;
            }

            if let Some(manager) = value {
                let model = gtk::NoSelection::new(Some(manager.to_owned()));
                self.connection_list_view.set_model(Some(&model));
            }

            self.connection_manager.set(value);
            obj.notify("connection-manager");
        }
    }
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

    fn switch_connection(&self, connection_manager: &model::ConnectionManager, uuid: &str) {
        connection_manager.set_client_from(
            uuid,
            clone!(@weak self as obj => move |result| if let Err(e) = result { obj.on_error(e); }),
        );
    }
}
