use crate::engine;

pub(crate) enum Exec {
    Docker { docker: bollard::Docker, id: String },
    Podman(podman_api::api::Exec),
}

impl Exec {
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Docker { id, .. } => id,
            Self::Podman(exec) => exec.id().as_ref(),
        }
    }
}

impl Exec {
    pub(crate) async fn resize(&self, width: usize, height: usize) -> anyhow::Result<()> {
        match self {
            Self::Docker { docker, id } => docker
                .resize_exec(
                    id,
                    bollard::query_parameters::ResizeExecOptions {
                        w: width as i32,
                        h: height as i32,
                    },
                )
                .await
                .map_err(anyhow::Error::from),
            Self::Podman(exec) => exec
                .resize(width, height)
                .await
                .map_err(anyhow::Error::from),
        }
    }

    pub(crate) async fn start(&self, tty: bool) -> anyhow::Result<engine::conn::ExecStart> {
        match self {
            Self::Docker { docker, id } => docker
                .start_exec(
                    id,
                    Some(bollard::exec::StartExecOptions {
                        tty,
                        ..Default::default()
                    }),
                )
                .await
                .map_err(anyhow::Error::from)
                .map(Into::into),
            Self::Podman(exec) => exec
                .start(&podman_api::opts::ExecStartOpts::builder().tty(tty).build())
                .await
                .map_err(anyhow::Error::from)
                .map(Into::into),
        }
    }
}
