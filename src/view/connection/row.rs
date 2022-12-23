use gettextrs::gettext;
use glib::subclass::InitializingObject;
use gtk::glib;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/connection/row.ui")]
    pub(crate) struct Row {
        pub(super) client: glib::WeakRef<model::Client>,
        pub(super) connection: glib::WeakRef<model::Connection>,
        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) checkmark: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) url_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) end_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) delete_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsConnectionRow";
        type Type = super::Row;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Row {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Client>("client")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecObject::builder::<model::Connection>("connection")
                        .explicit_notify()
                        .build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "client" => obj.set_client(value.get().unwrap()),
                "connection" => obj.set_connection(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "client" => obj.client().to_value(),
                "connection" => obj.connection().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let connection_expr = Self::Type::this_expression("connection");
            let is_remote_expr = connection_expr.chain_property::<model::Connection>("is-remote");

            is_remote_expr
                .chain_closure::<String>(closure!(|_: Self::Type, is_remote: bool| {
                    if is_remote {
                        "network-server-symbolic"
                    } else {
                        "computer-symbolic"
                    }
                }))
                .bind(&*self.image, "icon-name", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [
                    &is_remote_expr,
                    &connection_expr.chain_property::<model::Connection>("url"),
                ],
                closure!(|_: Self::Type, is_remote: bool, url: String| {
                    if is_remote {
                        url
                    } else {
                        gettext("Local connection")
                    }
                }),
            )
            .bind(&*self.url_label, "label", Some(obj));

            let is_active_expr = gtk::ClosureExpression::new::<bool>(
                [
                    &connection_expr,
                    &connection_expr
                        .chain_property::<model::Connection>("manager")
                        .chain_property::<model::ConnectionManager>("client"),
                ],
                closure!(|_: Self::Type,
                          connection: Option<model::Connection>,
                          _: Option<model::Client>| {
                    connection
                        .map(|connection| connection.is_active())
                        .unwrap_or(false)
                }),
            );

            let classes = self.image.css_classes();
            is_active_expr
                .chain_closure::<Vec<String>>(closure!(|_: Self::Type, is_active: bool| {
                    classes
                        .iter()
                        .cloned()
                        .chain(Some(glib::GString::from(if is_active {
                            "selected-connection"
                        } else {
                            "unselected-connection"
                        })))
                        .collect::<Vec<_>>()
                }))
                .bind(&*self.image, "css-classes", Some(obj));

            is_active_expr.bind(&*self.checkmark, "visible", Some(obj));

            connection_expr
                .chain_property::<model::Connection>("connecting")
                .chain_closure::<String>(closure!(
                    |_: Self::Type, connecting: bool| if connecting {
                        "connecting"
                    } else {
                        "delete"
                    }
                ))
                .bind(&*self.end_stack, "visible-child-name", Some(obj));

            connection_expr
                .chain_property::<model::Connection>("uuid")
                .chain_closure::<Option<glib::Variant>>(closure!(|_: Self::Type, uuid: &str| {
                    Some(uuid.to_variant())
                }))
                .bind(&*self.delete_button, "action-target", Some(obj));
        }

        fn dispose(&self) {
            utils::ChildIter::from(self.obj().upcast_ref()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for Row {}
}

glib::wrapper! {
    pub(crate) struct Row(ObjectSubclass<imp::Row>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Row {
    pub(crate) fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    pub(crate) fn set_client(&self, value: Option<&model::Client>) {
        if self.client().as_ref() == value {
            return;
        }
        self.imp().client.set(value);
        self.notify("client");
    }

    pub(crate) fn connection(&self) -> Option<model::Connection> {
        self.imp().connection.upgrade()
    }

    pub(crate) fn set_connection(&self, value: Option<&model::Connection>) {
        if self.connection().as_ref() == value {
            return;
        }
        self.imp().connection.set(value);
        self.notify("connection");
    }
}
