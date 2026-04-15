#[derive(Default)]
pub(crate) struct ExecCreateOpts {
    pub(crate) attach_stderr: Option<bool>,
    pub(crate) attach_stdin: Option<bool>,
    pub(crate) attach_stdout: Option<bool>,
    pub(crate) command: Vec<String>,
    pub(crate) tty: Option<bool>,
}

impl From<ExecCreateOpts> for bollard::config::ExecConfig {
    fn from(value: ExecCreateOpts) -> Self {
        Self {
            attach_stdout: value.attach_stdout,
            attach_stderr: value.attach_stderr,
            attach_stdin: value.attach_stdin,
            tty: value.tty,
            cmd: Some(value.command),
            ..Default::default()
        }
    }
}

impl From<ExecCreateOpts> for podman_api::opts::ExecCreateOpts {
    fn from(value: ExecCreateOpts) -> podman_api::opts::ExecCreateOpts {
        let mut builder = Self::builder().command(value.command);

        if let Some(attach_stderr) = value.attach_stderr {
            builder = builder.attach_stderr(attach_stderr);
        }
        if let Some(attach_stdin) = value.attach_stdin {
            builder = builder.attach_stdin(attach_stdin);
        }
        if let Some(attach_stdout) = value.attach_stdout {
            builder = builder.attach_stdout(attach_stdout);
        }
        if let Some(tty) = value.tty {
            builder = builder.tty(tty);
        }

        builder.build()
    }
}
