use std::path::{Path, PathBuf};

use http_body_util::{BodyExt, Full};
use hyper::{Uri, body::Bytes};
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use hyperlocal::{UnixConnector, Uri as LLUri};
use serde_json::Value;

#[derive(Clone)]
pub enum Podman2 {
    Unix {
        client: Client<UnixConnector, Full<Bytes>>,
        socket: PathBuf,
    },
    Tcp(Client<HttpConnector, Full<Bytes>>),
}

impl Podman2 {
    fn make_uri(&self, path: &str) -> Uri {
        match self {
            Self::Unix { client, socket } => LLUri::new(socket, path).into(),
            Self::Tcp(client) => {
                unimplemented!()
            }
        }
    }

    async fn get(&self, path: &str) -> Value {
        self.request(path).await
    }

    async fn post(&self, path: &str) -> Value {
        self.request(path).await
    }

    async fn request(&self, path: &str) -> Value {
        let uri = self.make_uri(path);
        let response = match self {
            Self::Unix { client, .. } => client.get(uri.into()).await,
            Self::Tcp(client) => client.get(uri).await,
        }
        .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();

        println!("{:?}", bytes);

        serde_json::from_slice(&bytes).unwrap()
    }
}

impl Podman2 {
    pub fn unix<P: AsRef<Path> + Into<PathBuf>>(socket: P) -> Self {
        Self::Unix {
            client: Client::builder(TokioExecutor::new()).build(UnixConnector),
            socket: socket.into(),
        }
    }

    pub fn system(&self) -> System {
        System(self.clone())
    }
}

pub struct System(Podman2);

impl System {
    pub async fn info(&self) -> Value {
        self.0.request("libpod/info").await
    }
}
