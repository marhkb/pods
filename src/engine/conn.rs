use std::pin::Pin;

use bytes::Bytes;
use futures::AsyncWrite;
use futures::Stream;
use futures::StreamExt;
use futures::TryStreamExt;
use tokio_util::compat::TokioAsyncWriteCompatExt;

pub(crate) enum ExecStart {
    Attached(Multiplexer),
    Detached,
}

impl ExecStart {
    pub(crate) fn into_attached(self) -> Option<Multiplexer> {
        match self {
            Self::Attached(multiplexer) => Some(multiplexer),
            Self::Detached => None,
        }
    }
}

impl From<bollard::exec::StartExecResults> for ExecStart {
    fn from(value: bollard::exec::StartExecResults) -> Self {
        match value {
            bollard::exec::StartExecResults::Attached { output, input } => {
                Self::Attached(Multiplexer {
                    reader: output
                        .map_ok(Into::into)
                        .map_err(anyhow::Error::from)
                        .boxed(),
                    writer: Box::pin(input.compat_write()),
                })
            }
            bollard::exec::StartExecResults::Detached => Self::Detached,
        }
    }
}

impl From<Option<podman_api::conn::Multiplexer>> for ExecStart {
    fn from(value: Option<podman_api::conn::Multiplexer>) -> Self {
        value
            .map(|multiplexer| {
                let (output, input) = multiplexer.split();
                Self::Attached(Multiplexer {
                    reader: output
                        .map_err(anyhow::Error::from)
                        .map_ok(Into::into)
                        .boxed(),
                    writer: Box::pin(input),
                })
            })
            .unwrap_or(Self::Detached)
    }
}

pub(crate) struct Multiplexer {
    reader: Pin<Box<dyn Stream<Item = anyhow::Result<TtyChunk>> + Send>>,
    writer: Pin<Box<dyn AsyncWrite + Send>>,
}

impl Multiplexer {
    #[allow(clippy::type_complexity)]
    pub(crate) fn split(
        self,
    ) -> (
        Pin<Box<dyn Stream<Item = anyhow::Result<TtyChunk>> + Send>>,
        Pin<Box<dyn AsyncWrite + Send>>,
    ) {
        (self.reader, self.writer)
    }
}

#[allow(clippy::enum_variant_names)]
pub(crate) enum TtyChunk {
    StdIn(Bytes),
    StdOut(Bytes),
    StdErr(Bytes),
}

impl From<TtyChunk> for Bytes {
    fn from(value: TtyChunk) -> Self {
        match value {
            TtyChunk::StdIn(buf) | TtyChunk::StdOut(buf) | TtyChunk::StdErr(buf) => buf,
        }
    }
}

impl AsRef<[u8]> for TtyChunk {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::StdIn(buf) | Self::StdOut(buf) | Self::StdErr(buf) => buf,
        }
    }
}

impl From<bollard::container::LogOutput> for TtyChunk {
    fn from(value: bollard::container::LogOutput) -> Self {
        match value {
            bollard::container::LogOutput::StdErr { message }
            | bollard::container::LogOutput::Console { message } => Self::StdErr(message),
            bollard::container::LogOutput::StdIn { message } => Self::StdIn(message),
            bollard::container::LogOutput::StdOut { message } => Self::StdOut(message),
        }
    }
}

impl From<podman_api::conn::TtyChunk> for TtyChunk {
    fn from(value: podman_api::conn::TtyChunk) -> Self {
        match value {
            podman_api::conn::TtyChunk::StdErr(buf) => Self::StdErr(Bytes::from(buf)),
            podman_api::conn::TtyChunk::StdIn(buf) => Self::StdIn(Bytes::from(buf)),
            podman_api::conn::TtyChunk::StdOut(buf) => Self::StdOut(Bytes::from(buf)),
        }
    }
}
