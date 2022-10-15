use std::cell::Cell;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::ObjectExt;
use gtk::prelude::StaticType;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;
use serde::Deserialize;
use serde::Serialize;

use crate::model;

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct ConnectionInfo {
    pub(super) uuid: String,
    pub(super) name: String,
    pub(super) url: String,
    pub(super) rgb: Option<(f32, f32, f32)>,
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct Connection {
        pub(super) manager: glib::WeakRef<model::ConnectionManager>,
        pub(super) uuid: OnceCell<String>,
        pub(super) name: OnceCell<String>,
        pub(super) url: OnceCell<String>,
        pub(super) rgb: Cell<Option<gdk::RGBA>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Connection {
        const NAME: &'static str = "Connection";
        type Type = super::Connection;
    }

    impl ObjectImpl for Connection {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "manager",
                        "Manager",
                        "The connection manager",
                        model::ConnectionManager::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "uuid",
                        "Uuid",
                        "the uuid of the connection",
                        None,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "name",
                        "Name",
                        "the name of the connection",
                        None,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "url",
                        "Url",
                        "The URL of the connection",
                        None,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecBoxed::new(
                        "rgb",
                        "Rgb",
                        "The rgb color of the connection",
                        gdk::RGBA::static_type(),
                        glib::ParamFlags::READWRITE
                            | glib::ParamFlags::CONSTRUCT
                            | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecBoolean::new(
                        "is-remote",
                        "Is Remote",
                        "Whether this is a remote connection",
                        false,
                        glib::ParamFlags::READABLE,
                    ),
                ]
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
                "manager" => self.manager.set(value.get().unwrap()),
                "uuid" => self.uuid.set(value.get().unwrap()).unwrap(),
                "name" => self.name.set(value.get().unwrap()).unwrap(),
                "url" => self.url.set(value.get().unwrap()).unwrap(),
                "rgb" => obj.set_rgb(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "manager" => obj.manager().to_value(),
                "uuid" => obj.uuid().to_value(),
                "name" => obj.name().to_value(),
                "url" => obj.url().to_value(),
                "rgb" => obj.rgb().to_value(),
                "is-remote" => obj.is_remote().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct Connection(ObjectSubclass<imp::Connection>);
}

impl From<&Connection> for ConnectionInfo {
    fn from(connection: &Connection) -> Self {
        Self {
            uuid: connection.uuid().to_string(),
            name: connection.name().to_string(),
            url: connection.url().to_owned(),
            rgb: connection
                .rgb()
                .map(|rgb| (rgb.red(), rgb.green(), rgb.blue())),
        }
    }
}

impl Connection {
    pub(crate) fn from_connection_info(
        connection_info: &ConnectionInfo,
        manager: &model::ConnectionManager,
    ) -> Self {
        Self::new(
            &connection_info.uuid,
            &connection_info.name,
            &connection_info.url,
            connection_info
                .rgb
                .map(|(r, g, b)| gdk::RGBA::new(r, g, b, 1.0)),
            manager,
        )
    }

    pub(crate) fn new(
        uuid: &str,
        name: &str,
        url: &str,
        rgb: Option<gdk::RGBA>,
        manager: &model::ConnectionManager,
    ) -> Self {
        glib::Object::new(&[
            ("manager", manager),
            ("uuid", &uuid),
            ("name", &name),
            ("url", &url),
            ("rgb", &rgb),
        ])
        .expect("Failed to create Connection")
    }

    pub(crate) fn manager(&self) -> Option<model::ConnectionManager> {
        self.imp().manager.upgrade()
    }

    pub(crate) fn uuid(&self) -> &str {
        self.imp().uuid.get().unwrap()
    }

    pub(crate) fn name(&self) -> &str {
        self.imp().name.get().unwrap()
    }

    pub(crate) fn url(&self) -> &str {
        self.imp().url.get().unwrap()
    }

    pub(crate) fn rgb(&self) -> Option<gdk::RGBA> {
        self.imp().rgb.get()
    }

    pub(crate) fn set_rgb(&self, value: Option<gdk::RGBA>) {
        if self.rgb() == value {
            return;
        }
        self.imp().rgb.set(value);
        self.notify("rgb");
    }

    pub(crate) fn is_local(&self) -> bool {
        self.url().starts_with("unix")
    }

    pub(crate) fn is_remote(&self) -> bool {
        !self.is_local()
    }

    pub(crate) fn is_active(&self) -> bool {
        self.manager()
            .as_ref()
            .and_then(model::ConnectionManager::client)
            .map(|client| client.connection() == self)
            .unwrap_or(false)
    }
}
