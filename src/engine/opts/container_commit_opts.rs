pub(crate) struct ContainerCommitOpts {
    pub(crate) author: Option<String>,
    pub(crate) comment: Option<String>,
    pub(crate) format: Option<String>,
    pub(crate) pause: bool,
    pub(crate) repo: Option<String>,
    pub(crate) tag: Option<String>,
}

impl From<ContainerCommitOpts> for bollard::query_parameters::CommitContainerOptions {
    fn from(value: ContainerCommitOpts) -> Self {
        Self {
            author: value.author,
            comment: value.comment,
            pause: value.pause,
            repo: value.repo,
            tag: value.tag,
            ..Default::default()
        }
    }
}

impl From<ContainerCommitOpts> for podman_api::opts::ContainerCommitOpts {
    fn from(value: ContainerCommitOpts) -> podman_api::opts::ContainerCommitOpts {
        let mut builder = podman_api::opts::ContainerCommitOpts::builder().pause(value.pause);

        if let Some(author) = value.author {
            builder = builder.author(author);
        }
        if let Some(comment) = value.comment {
            builder = builder.comment(comment);
        }
        if let Some(format) = value.format {
            builder = builder.format(format);
        }
        if let Some(repo) = value.repo {
            builder = builder.repo(repo);
        }
        if let Some(tag) = value.tag {
            builder = builder.tag(tag);
        }

        builder.build()
    }
}
