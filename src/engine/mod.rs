mod info;

use std::path::{Path, PathBuf};

use http_body_util::{BodyExt, Full};
use hyper::{Uri, body::Bytes};
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use hyperlocal::{UnixConnector, Uri as SocketUri};
use serde_json::Value;

use crate::engine::info::Info;

#[derive(Clone, Debug)]
pub enum Backend {
    Docker,
    Podman,
}

#[derive(Clone, Debug)]
pub struct Engine {
    backend: Backend,
    connection: Connection,
}

impl Engine {
    pub(crate) async fn new<U: AsRef<str>>(uri: U) -> anyhow::Result<Self> {
        let connection = Connection::new(uri)?;

        Ok(Self {
            backend: match connection.name().await.as_deref() {
                Some("Docker Engine") => Backend::Docker,
                Some("Podman Engine") => Backend::Podman,
                Some(other) => return Err(anyhow::anyhow!(other.to_owned())),
                _ => return Err(anyhow::anyhow!("todo")),
            },
            connection,
        })
    }

    pub async fn info(&self) -> Info {
        let response = self.connection.request(&self.make_path("info")).await;
        match self.backend {
            Backend::Docker => Info::docker(response),
            Backend::Podman => Info::podman(response),
        }
    }

    fn make_path(&self, path: &str) -> String {
        match self.backend {
            Backend::Docker => format!("d/v1.53/{path}"),
            Backend::Podman => format!("d/v5.0.0/libpod/{path}"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Connection {
    Unix {
        client: Client<UnixConnector, Full<Bytes>>,
        socket: PathBuf,
    },
    Tcp(Client<HttpConnector, Full<Bytes>>),
}

impl Connection {
    fn make_uri(&self, path: &str) -> Uri {
        match self {
            Self::Unix { socket, .. } => SocketUri::new(socket, &path).into(),
            Self::Tcp(..) => {
                unimplemented!()
            }
        }
    }

    pub async fn name(&self) -> Option<String> {
        let val = self.request("d/version").await;

        val.get("Components")
            .and_then(Value::as_array)
            .and_then(|array| array.iter().find_map(|value| value.get("Name")))
            .and_then(Value::as_str)
            .map(str::to_string)
    }

    async fn request(&self, path: &str) -> Value {
        let uri = self.make_uri(path);
        let response = match self {
            Self::Unix { client, .. } => client.get(uri.into()).await,
            Self::Tcp(client) => client.get(uri).await,
        }
        .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();

        serde_json::from_slice(&bytes).unwrap()
    }
}

impl Connection {
    pub fn new<U>(uri: U) -> anyhow::Result<Connection>
    where
        U: AsRef<str>,
    {
        let uri = uri.as_ref();
        let mut it = uri.split("://");

        match it.next() {
            Some("unix") => {
                if let Some(path) = it.next() {
                    Ok(Connection::unix(path))
                } else {
                    Err(anyhow::anyhow!("todo"))
                }
            }
            Some("tcp") | Some("http") => {
                if let Some(host) = it.next() {
                    todo!()
                    // Podman::tcp_versioned(host, version)
                } else {
                    todo!()
                    // Err(Error::MissingAuthority)
                }
            }
            Some(scheme) => Err(anyhow::anyhow!("todo")),
            None => unreachable!(),
        }
    }

    pub fn unix<P: AsRef<Path> + Into<PathBuf>>(socket: P) -> Self {
        Self::Unix {
            client: Client::builder(TokioExecutor::new()).build(UnixConnector),
            socket: socket.into(),
        }
    }
}
