use gettextrs::gettext;
use glib::subclass::InitializingObject;
use gtk::gio::{self};
use gtk::glib::clone;
use gtk::glib::WeakRef;
use gtk::glib::{self};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/connection-switcher-popup.ui")]
    pub(crate) struct ConnectionSwitcherPopup {
        pub(super) connection_manager: WeakRef<model::ConnectionManager>,
        #[template_child]
        pub(super) connection_list_view: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConnectionSwitcherPopup {
        const NAME: &'static str = "ConnectionSwitcherPopup";
        type Type = super::ConnectionSwitcherPopup;
        type ParentType = gtk::Popover;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.set_accessible_role(gtk::AccessibleRole::Dialog);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ConnectionSwitcherPopup {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "connection-manager",
                    "Connection Manager",
                    "The connection manager client",
                    model::ConnectionManager::static_type(),
                    glib::ParamFlags::READWRITE,
                )]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "connection-manager" => obj.set_connection_manager(value.get().unwrap()),
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

            self.connection_list_view.connect_activate(
                clone!(@weak obj => move |list_view, index| {
                    let connection = list_view
                        .model()
                        .unwrap()
                        .item(index)
                        .unwrap()
                        .downcast::<model::Connection>()
                        .unwrap();

                    if let Err(e) = obj.connection_manager().unwrap().set_client_from(
                        connection.uuid(),
                    ) {
                        obj.on_error(e);
                    }
                }),
            );
        }
    }

    impl WidgetImpl for ConnectionSwitcherPopup {}
    impl PopoverImpl for ConnectionSwitcherPopup {}
}

glib::wrapper! {
    pub(crate) struct ConnectionSwitcherPopup(ObjectSubclass<imp::ConnectionSwitcherPopup>)
        @extends gtk::Widget, gtk::Popover,
        @implements gtk::Accessible, gio::ListModel;
}

impl ConnectionSwitcherPopup {
    fn on_error(&self, e: impl ToString) {
        utils::show_error_toast(
            self,
            &gettext("Error on swwitching connection"),
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
}
