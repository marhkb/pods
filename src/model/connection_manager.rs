use std::cell::Cell;
use std::cell::RefCell;
use std::io::Read;
use std::marker::PhantomData;
use std::path::PathBuf;

use futures::future;
use gettextrs::gettext;
use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use indexmap::IndexMap;
use tokio::io::AsyncWriteExt;

use crate::engine;
use crate::model;
use crate::rt;
use crate::utils;
use crate::utils::config_dir;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ConnectionManager)]
    pub(crate) struct ConnectionManager {
        pub(super) settings: utils::PodsSettings,
        pub(super) connections: RefCell<IndexMap<String, model::Connection>>,
        pub(super) connect_abort_handle: RefCell<Option<future::AbortHandle>>,

        #[property(get)]
        pub(super) client: RefCell<Option<model::Client>>,
        #[property(get, set)]
        pub(super) creating_new_connection: Cell<bool>,

        #[property(get = Self::connecting)]
        _connecting: PhantomData<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConnectionManager {
        const NAME: &'static str = "ConnectionManager";
        type Type = super::ConnectionManager;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for ConnectionManager {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }
    }

    impl ListModelImpl for ConnectionManager {
        fn item_type(&self) -> glib::Type {
            model::Connection::static_type()
        }

        fn n_items(&self) -> u32 {
            self.connections.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.connections
                .borrow()
                .get_index(position as usize)
                .map(|(_, obj)| obj.upcast_ref())
                .cloned()
        }
    }

    impl ConnectionManager {
        pub(crate) fn connecting(&self) -> bool {
            self.obj().creating_new_connection()
                || self
                    .connections
                    .borrow()
                    .values()
                    .any(model::Connection::connecting)
        }
    }
}

glib::wrapper! {
    pub(crate) struct ConnectionManager(ObjectSubclass<imp::ConnectionManager>)
        @implements gio::ListModel;
}

impl Default for ConnectionManager {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ConnectionManager {
    pub(crate) fn setup<F>(&self, op: F)
    where
        F: Fn(anyhow::Error) + 'static,
    {
        let connections = match self.load_from_disk() {
            Ok(connections) => connections,
            Err(e) => {
                op(e);
                return;
            }
        };
        let connections_len = connections.len();

        let imp = self.imp();

        imp.connections.borrow_mut().extend(
            connections
                .into_iter()
                .map(|(uuid, conn)| (uuid, model::Connection::from_connection_info(&conn, self))),
        );

        self.items_changed(
            (imp.connections.borrow().len() - connections_len) as u32,
            0,
            connections_len as u32,
        );

        if self.n_items() > 0 {
            let last_used_connection = imp.settings.string("last-used-connection");
            self.set_client_from(last_used_connection.as_str(), op);
        }
    }

    fn load_from_disk(&self) -> anyhow::Result<IndexMap<String, model::ConnectionInfo>> {
        if utils::config_dir().exists() {
            let path = path();

            if path.exists() {
                let mut file = std::fs::OpenOptions::new().read(true).open(path)?;

                let mut buf = vec![];
                file.read_to_end(&mut buf)?;

                serde_json::from_slice::<IndexMap<String, model::ConnectionInfo>>(&buf)
                    .map_err(anyhow::Error::from)
            } else {
                Ok(IndexMap::default())
            }
        } else {
            std::fs::create_dir_all(config_dir())?;
            Ok(IndexMap::default())
        }
    }

    pub(crate) async fn sync_to_disk(&self) -> anyhow::Result<()> {
        let value = self
            .imp()
            .connections
            .borrow()
            .iter()
            .map(|(key, connection)| (key.to_owned(), model::ConnectionInfo::from(connection)))
            .collect::<IndexMap<_, _>>();

        let buf = serde_json::to_vec_pretty(&value).unwrap();

        rt::Promise::new(async move {
            if !utils::config_dir().exists() {
                tokio::fs::create_dir_all(&config_dir()).await?;
            }

            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path())
                .await?;

            file.write_all(&buf).await.map_err(anyhow::Error::from)
        })
        .exec()
        .await
        .inspect_err(|e| log::error!("Failed to sync connections to disk: {e}"))
    }

    pub(crate) async fn try_connect(
        &self,
        name: &str,
        url: &str,
        rgb: Option<gdk::RGBA>,
    ) -> Option<anyhow::Result<()>> {
        let imp = self.imp();

        if imp.connections.borrow().values().any(|c| c.name() == name) {
            return Some(Err(anyhow::anyhow!(gettext!(
                "Connection '{}' already exists",
                name
            ))));
        }

        let connection =
            model::Connection::new(glib::uuid_string_random().as_str(), name, url, rgb, self);

        self.set_creating_new_connection(true);

        let engine = create_engine(self.abort_registration(), connection.url().to_owned())
            .exec()
            .await?;

        self.set_creating_new_connection(false);

        match engine {
            Ok(engine) => {
                let (position, _) = imp
                    .connections
                    .borrow_mut()
                    .insert_full(connection.uuid(), connection.clone());

                self.items_changed(position as u32, 0, 1);

                self.set_client(Some(model::Client::new(&connection, engine)));

                _ = self.sync_to_disk().await;

                Some(Ok(()))
            }
            Err(e) => {
                log::error!("connection error: {e}");
                Some(Err(e))
            }
        }
    }

    pub(crate) async fn remove_connection(&self, uuid: &str) {
        let Some((position, _, _)) = self.imp().connections.borrow_mut().shift_remove_full(uuid)
        else {
            return;
        };

        self.items_changed(position as u32, 1, 0);

        if self
            .client()
            .map(|client| client.connection().uuid() == uuid)
            .unwrap_or(false)
        {
            self.set_client(None);
        }

        _ = self.sync_to_disk().await;
    }

    pub(crate) fn contains_local_connection(&self) -> bool {
        self.imp()
            .connections
            .borrow()
            .values()
            .any(model::Connection::is_local)
    }

    pub(crate) fn set_client_from<F>(&self, connection_uuid: &str, op: F)
    where
        F: Fn(anyhow::Error) + 'static,
    {
        if self
            .client()
            .map(|c| c.connection().uuid() == connection_uuid)
            .unwrap_or(false)
        {
            log::error!("Connection not found: {connection_uuid}");
            return;
        }

        let connection = match self
            .connection_by_uuid(connection_uuid)
            .ok_or_else(|| anyhow::anyhow!("connection not found"))
        {
            Ok(connection) => connection,
            Err(e) => {
                log::error!("Failed to connect: {}", e);
                op(e);
                return;
            }
        };

        if connection.connecting() {
            return;
        }

        connection.set_connecting(true);

        create_engine(self.abort_registration(), connection.url().to_owned()).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            #[weak]
            connection,
            move |engine| {
                if let Some(engine) = engine {
                    match engine {
                        Ok(engine) => {
                            obj.set_client(Some(model::Client::new(&connection, engine)));
                        }
                        Err(e) => {
                            log::error!("Failed to connect: {}", e);
                            op(e);
                        }
                    }
                }

                connection.set_connecting(false);
            }
        ));
    }

    fn set_client(&self, value: Option<model::Client>) {
        if self.client() == value {
            return;
        }

        if let Some(client) = self.client() {
            client.connection().set_active(false);
        }

        let imp = self.imp();

        if let Some(ref client) = value {
            client.connection().set_active(true);

            if let Err(e) = imp
                .settings
                .set_string("last-used-connection", &client.connection().uuid())
            {
                log::error!("Could not write last used connection {e}");
            }
        }

        imp.client.replace(value);
        self.notify_client();
    }

    pub(crate) fn unset_client(&self) {
        self.set_client(None);
    }

    fn abort_registration(&self) -> future::AbortRegistration {
        self.abort();

        let (abort_handle, abort_registration) = future::AbortHandle::new_pair();
        self.imp().connect_abort_handle.replace(Some(abort_handle));

        abort_registration
    }

    pub(crate) fn abort(&self) {
        if let Some(abort_handle) = self.imp().connect_abort_handle.take() {
            abort_handle.abort();
        }
    }

    pub(crate) fn connection_by_uuid(&self, uuid: &str) -> Option<model::Connection> {
        self.imp().connections.borrow().get(uuid).cloned()
    }

    pub(crate) fn position_by_uuid(&self, uuid: &str) -> u32 {
        self.imp()
            .connections
            .borrow()
            .get_index_of(uuid)
            .map(|position| position as u32)
            .unwrap_or(gtk::INVALID_LIST_POSITION)
    }
}

fn path() -> PathBuf {
    utils::config_dir().join("connections.json")
}

fn create_engine(
    abort_registration: future::AbortRegistration,
    url: String,
) -> rt::Promise<impl Future<Output = Option<anyhow::Result<engine::Engine>>>> {
    rt::Promise::new(async move {
        future::Abortable::new(engine::Engine::new(url), abort_registration)
            .await
            .ok()
    })
}
