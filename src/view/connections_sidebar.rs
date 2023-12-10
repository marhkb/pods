use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ConnectionsSidebar)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/connections_sidebar.ui")]
    pub(crate) struct ConnectionsSidebar {
        #[property(get, set, nullable)]
        pub(super) connection_manager: glib::WeakRef<model::ConnectionManager>,
        #[template_child]
        pub(super) selection: TemplateChild<gtk::SingleSelection>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConnectionsSidebar {
        const NAME: &'static str = "PdsConnectionsSidebar";
        type Type = super::ConnectionsSidebar;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
            klass.set_css_name("connectionssidebar");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ConnectionsSidebar {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            Self::Type::this_expression("connection-manager")
                .chain_property::<model::ConnectionManager>("client")
                .chain_closure::<u32>(closure!(|_: Self::Type, client: Option<model::Client>| {
                    client
                        .map(|client| client.connection().position())
                        .unwrap_or(gtk::INVALID_LIST_POSITION)
                }))
                .bind(&self.selection.get(), "selected", Some(&*self.obj()));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ConnectionsSidebar {}

    #[gtk::template_callbacks]
    impl ConnectionsSidebar {
        #[template_callback]
        fn on_list_view_activated(&self, position: u32) {
            let obj = &*self.obj();
            if let Some(connection_manager) = obj.connection_manager() {
                let connection = self
                    .selection
                    .item(position)
                    .unwrap()
                    .downcast::<model::Connection>()
                    .unwrap();

                if connection.is_active() {
                    return;
                }

                if view::show_ongoing_actions_warning_dialog(
                    obj.upcast_ref(),
                    &connection_manager,
                    &gettext("Confirm Switching Connection"),
                ) {
                    connection_manager.set_client_from(
                        &connection.uuid(),
                        clone!(@weak obj => move |result| if let Err(e) = result {
                            utils::show_error_toast(
                                obj.upcast_ref(),
                                &gettext("Error on switching connection"),
                                &e.to_string(),
                            );
                        }),
                    );
                }
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct ConnectionsSidebar(ObjectSubclass<imp::ConnectionsSidebar>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
