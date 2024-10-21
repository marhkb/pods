use std::borrow::Cow;
use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::Arc;

use adw::prelude::*;
use futures::lock::Mutex;
use futures::stream;
use futures::FutureExt;
use futures::StreamExt;
use futures::TryFutureExt;
use futures::TryStreamExt;
use gettextrs::gettext;
use gio::subclass::prelude::*;
use glib::clone;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use tokio::io::BufWriter;

use crate::model;
use crate::model::AbstractContainerListExt;
use crate::podman;
use crate::utils;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ActionState")]
pub(crate) enum State {
    #[default]
    Ongoing,
    Finished,
    Aborted,
    Failed,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ActionType")]
pub(crate) enum Type {
    PruneImages,
    DownloadImage,
    BuildImage,
    PushImage,
    PruneContainers,
    Commit,
    CreateContainer,
    CreateAndRunContainer,
    CopyFiles,
    PrunePods,
    Pod,
    Volume,
    PruneVolumes,
    #[default]
    Undefined,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Action)]
    pub(crate) struct Action {
        pub(super) abort_handle: RefCell<Option<stream::AbortHandle>>,
        #[property(get, nullable)]
        pub(super) artifact: glib::WeakRef<glib::Object>,
        #[property(get, set, construct_only)]
        pub(super) num: OnceCell<u32>,
        #[property(get, set, construct_only, builder(Type::default()))]
        pub(super) action_type: OnceCell<Type>,
        #[property(get, set, construct_only)]
        pub(super) description: OnceCell<String>,
        #[property(get, builder(State::default()))]
        pub(super) state: Cell<State>,
        #[property(get, set, construct_only)]
        pub(super) start_timestamp: OnceCell<i64>,
        #[property(get)]
        pub(super) end_timestamp: OnceCell<i64>,
        #[property(get)]
        pub(super) output: gtk::TextBuffer,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Action {
        const NAME: &'static str = "Action";
        type Type = super::Action;
    }

    impl ObjectImpl for Action {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }
}

glib::wrapper! {
    pub(crate) struct Action(ObjectSubclass<imp::Action>);
}

impl Action {
    pub(crate) fn cancel(&self) {
        if let Some(handle) = &*self.imp().abort_handle.borrow() {
            handle.abort();
            self.set_state(State::Aborted);
        }
    }
}

impl Action {
    fn new(num: u32, type_: Type, description: &str) -> Self {
        glib::Object::builder()
            .property("num", num)
            .property("action-type", type_)
            .property("description", description)
            .property(
                "start-timestamp",
                glib::DateTime::now_local().unwrap().to_unix(),
            )
            .build()
    }

    pub(crate) fn prune_images(
        num: u32,
        client: model::Client,
        opts: podman::opts::ImagePruneOpts,
    ) -> Self {
        let obj = Self::new(num, Type::PruneImages, &gettext("Prune unused images"));
        let abort_registration = obj.setup_abort_handle();

        utils::do_async(
            {
                let podman = client.podman();
                async move {
                    stream::Abortable::new(podman.images().prune(&opts), abort_registration).await
                }
            },
            clone!(@weak obj => move |result| if let Ok(result) = result {
                let output = obj.output();
                let mut start_iter = output.start_iter();
                match result.as_ref() {
                    Ok(report) => {
                        output.insert(&mut start_iter, &serde_json::to_string_pretty(&report).unwrap());
                        obj.set_state(State::Finished);
                    },
                    Err(e) => {
                        output.insert(&mut start_iter, &e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }),
        );

        obj
    }

    pub(crate) fn download_image(
        num: u32,
        image: &str,
        client: model::Client,
        opts: podman::opts::PullOpts,
    ) -> Self {
        Self::new(
            num,
            Type::DownloadImage,
            &gettext!("Pull image <b>{}</b>", image),
        )
        .download_image_(client, opts, |obj, client, report| {
            let image_id = report.images.unwrap().swap_remove(0);
            match client.image_list().get_image(&image_id) {
                Some(image) => {
                    obj.set_artifact(image.upcast_ref());
                    obj.set_state(State::Finished);
                }
                None => {
                    client
                        .image_list()
                        .connect_image_added(clone!(@weak obj => move |_, image| {
                            if image.id() == image_id.as_str() {
                                obj.set_artifact(image.upcast_ref());
                                obj.set_state(State::Finished);
                            }
                        }));
                }
            }
        })
    }

    pub(crate) fn push_image(
        num: u32,
        destination: &str,
        image: podman::api::Image,
        opts: podman::opts::ImagePushOpts,
    ) -> Self {
        #[derive(Deserialize)]
        struct Report {
            stream: Option<String>,
            error: Option<String>,
        }

        let obj = Self::new(
            num,
            Type::PushImage,
            &gettext!("Push image <b>{}</b>", destination),
        );
        let abort_registration = obj.setup_abort_handle();

        utils::run_stream_with_finish_handler(
            image,
            move |image| stream::Abortable::new(image.push(&opts), abort_registration).boxed(),
            clone!(
                @weak obj => @default-return glib::ControlFlow::Break,
                move |result: podman::Result<String>|
            {
                match result.map_err(anyhow::Error::from).and_then(|line| {
                    serde_json::from_str::<Report>(&line).map_err(anyhow::Error::from)
                }) {
                    Ok(report) => match report.stream {
                        Some(line) => {
                            obj.insert(&line);
                            glib::ControlFlow::Continue
                        }
                        None => {
                            if let Some(line) = report.error {
                                log::error!("Error on pushing image: {line}");
                                obj.insert(&line);
                            }
                            obj.set_state(State::Failed);
                            glib::ControlFlow::Break
                        }
                    },
                    Err(e) => {
                        log::error!("Error on pushing image: {e}");
                        obj.insert_line(&e.to_string());
                        obj.set_state(State::Failed);
                        glib::ControlFlow::Break
                    }
                }
            }),
            clone!(@weak obj => move || {
                if obj.state() != State::Failed {
                    obj.set_state(State::Finished);
                }
            }),
        );

        obj
    }

    pub(crate) fn build_image(
        num: u32,
        image: &str,
        client: model::Client,
        opts: podman::opts::ImageBuildOpts,
    ) -> Self {
        let obj = Self::new(
            num,
            Type::BuildImage,
            &gettext!("Build image <b>{}</b>", image),
        );
        let abort_registration = obj.setup_abort_handle();

        obj.insert_line(&gettext("Generating tarball of context directory..."));

        utils::run_stream_with_finish_handler(
            client.podman().images(),
            move |images| match images.build(&opts) {
                Ok(stream) => stream::Abortable::new(stream, abort_registration).boxed(),
                Err(e) => {
                    log::error!("Error on building image: {e}");
                    futures::stream::empty().boxed()
                }
            },
            clone!(
                @weak obj, @weak client => @default-return glib::ControlFlow::Break,
                move |result: podman::Result<podman::models::ImageBuildLibpod200Response>|
            {
                match result {
                    Ok(stream) => {
                        obj.insert(&stream.stream);
                        glib::ControlFlow::Continue
                    }
                    Err(e) => {
                        log::error!("Error on building image: {e}");
                        obj.insert_line(&e.to_string());
                        obj.set_state(State::Failed);
                        glib::ControlFlow::Break
                    },
                }
            }),
            clone!(@weak obj, @weak client => move || {
                let output = obj.output();

                let start = output.iter_at_line(0).unwrap();
                let mut end = output.iter_at_line(0).unwrap();
                end.forward_to_line_end();

                let image_id = output.text(&start, &end, false).trim().to_owned();

                match client.image_list().get_image(&image_id) {
                    Some(image) => {
                        obj.set_artifact(image.upcast_ref());
                        obj.set_state(State::Finished);
                    }
                    None => {
                        client
                            .image_list()
                            .connect_image_added(clone!(@weak obj => move |_, image| {
                                if image.id() == image_id {
                                    obj.set_artifact(image.upcast_ref());
                                    obj.set_state(State::Finished);
                                }
                            }));
                    }
                }
            }),
        );

        obj
    }

    pub(crate) fn prune_containers(
        num: u32,
        client: model::Client,
        opts: podman::opts::ContainerPruneOpts,
    ) -> Self {
        let obj = Self::new(
            num,
            Type::PruneContainers,
            &gettext("Prune stopped containers"),
        );
        let abort_registration = obj.setup_abort_handle();

        utils::do_async(
            {
                let podman = client.podman();
                async move {
                    stream::Abortable::new(podman.containers().prune(&opts), abort_registration)
                        .await
                }
            },
            clone!(@weak obj => move |result| if let Ok(result) = result {
                let output = obj.output();
                let mut start_iter = output.start_iter();
                match result.as_ref() {
                    Ok(report) => {
                        output.insert(&mut start_iter, &serde_json::to_string_pretty(&report).unwrap());
                        obj.set_state(State::Finished);
                    },
                    Err(e) => {
                        output.insert(&mut start_iter, &e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }),
        );

        obj
    }

    fn container(num: u32, container: &str, run: bool) -> Self {
        Self::new(
            num,
            if run {
                Type::CreateAndRunContainer
            } else {
                Type::CreateContainer
            },
            &if run {
                gettext!("Start new container <b>{}</b>", container)
            } else {
                gettext!("Create container <b>{}</b>", container)
            },
        )
    }

    pub(crate) fn create_container(
        num: u32,
        container: &str,
        client: model::Client,
        opts: podman::opts::ContainerCreateOpts,
        run: bool,
    ) -> Self {
        Self::container(num, container, run).create_container_(client, opts, run)
    }

    pub(crate) fn commit_container(
        num: u32,
        image: Option<&str>,
        container: &str,
        api: podman::api::Container,
        opts: podman::opts::ContainerCommitOpts,
    ) -> Self {
        let obj = Self::new(
            num,
            Type::Commit,
            &gettext!(
                "Commit image <b>{}</b> ({})",
                container,
                image
                    .map(Cow::Borrowed)
                    .unwrap_or_else(|| Cow::Owned(format!("<i>&lt;{}&gt;</i>", gettext("none")))),
            ),
        );
        let abort_registration = obj.setup_abort_handle();

        utils::do_async(
            async move { stream::Abortable::new(api.commit(&opts), abort_registration).await },
            clone!(@weak obj => move |result| if let Ok(result) = result {
                match result.as_ref() {
                    Ok(_) => {
                        obj.insert_line(&gettext("Finished"));
                        obj.set_state(State::Finished);
                    },
                    Err(e) => {
                        obj.insert_line(&e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }),
        );

        obj
    }

    pub(crate) fn create_container_download_image(
        num: u32,
        container: &str,
        client: model::Client,
        pull_opts: podman::opts::PullOpts,
        create_opts_builder: podman::opts::ContainerCreateOptsBuilder,
        run: bool,
    ) -> Self {
        Self::container(num, container, run).download_image_(
            client,
            pull_opts,
            move |obj, client, report| {
                obj.create_container_(client, create_opts_builder.image(report.id).build(), run);
            },
        )
    }

    pub(crate) fn copy_files_into_container(
        num: u32,
        host_path: String,
        container_path: String,
        directory: bool,
        container: &model::Container,
    ) -> Self {
        let obj = Self::new(
            num,
            Type::CopyFiles,
            &gettext!(
                "Upload <b>{}</b> to <b>{}:{}</b>",
                host_path,
                container.name(),
                container_path,
            ),
        );

        obj.insert_line(&gettext("Creating tar archive…"));

        let abort_registration = obj.setup_abort_handle();

        utils::do_async(
            async move {
                let mut ar = tokio_tar::Builder::new(Vec::new());

                stream::Abortable::new(
                    async move {
                        let host_path = PathBuf::from(host_path);
                        let file_name = host_path.file_name();
                        if directory {
                            ar.append_dir_all(
                                file_name.unwrap_or_else(|| OsStr::new(".")),
                                &host_path,
                            )
                            .await
                        } else {
                            match tokio::fs::File::open(&host_path).await {
                                Ok(mut file) => ar.append_file(file_name.unwrap(), &mut file).await,
                                Err(e) => Err(e),
                            }
                        }
                        .map(|_| ar)
                    },
                    abort_registration,
                )
                .await
            },
            clone!(@weak obj, @weak container => move |result| if let Ok(result) = result {
                match result {
                    Ok(ar) => {
                        obj.insert_line(&gettext("Tar archive created"));
                        obj.insert_line(&gettext("Unwrapping tar archive…"));

                        let abort_registration = obj.setup_abort_handle();
                        utils::do_async(
                            stream::Abortable::new(ar.into_inner(), abort_registration),
                            clone!(@weak obj, @weak container => move |result| if let Ok(result) = result {
                                match result {
                                    Ok(data) => {
                                        obj.insert_line(&gettext("Tar archive unwrapped"));
                                        obj.insert_line(&gettext("Copying files into container…"));

                                        let abort_registration = obj.setup_abort_handle();
                                        let api = container.api().unwrap();
                                        utils::do_async(
                                            async move {
                                                stream::Abortable::new(
                                                    api.copy_to(container_path, data.into()),
                                                    abort_registration
                                                ).await
                                            },
                                            clone!(@weak obj => move |result| match result {
                                                Ok(_) => {
                                                    obj.insert_line(&gettext("Finished"));
                                                    obj.set_state(State::Finished);
                                                }
                                                Err(e) => {
                                                    obj.insert_line(&e.to_string());
                                                    obj.set_state(State::Failed);
                                                }
                                            }),
                                        );
                                    }
                                    Err(e) => {
                                        obj.insert_line(&e.to_string());
                                        obj.set_state(State::Failed);
                                    }
                                }
                            })
                        );
                    }
                    Err(e) => {
                        obj.insert_line(&e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }),
        );

        obj
    }

    pub(crate) fn copy_files_from_container(
        num: u32,
        container: &model::Container,
        container_path: String,
        host_path: String,
    ) -> Self {
        let obj = Self::new(
            num,
            Type::CopyFiles,
            &gettext!(
                "Download <b>{}:{}</b> to <b>{}</b>",
                container.name(),
                container_path,
                host_path
            ),
        );

        let abort_registration = obj.setup_abort_handle();

        obj.insert_line(&gettext("Writing to file…"));

        utils::do_async(
            async move {
                tokio::fs::File::options()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&host_path)
                    .await
            },
            clone!(
                #[weak]
                obj,
                #[weak]
                container,
                move |file| {
                    match file {
                        Err(e) => {
                            obj.insert_line(&e.to_string());
                            obj.set_state(State::Failed);
                        }
                        Ok(file) => {
                            let writer = Arc::new(Mutex::new(BufWriter::new(file)));
                            obj.insert_line(&gettext!("Written: {}", glib::format_size(0)));

                            utils::run_stream_with_finish_handler(
                                container.api().unwrap(),
                                {
                                    let writer = writer.clone();
                                    move |container| {
                                        stream::Abortable::new(
                                            container.copy_from(container_path),
                                            abort_registration,
                                        )
                                        .map_err(anyhow::Error::from)
                                        .scan(
                                            Ok((writer, 0)),
                                            |state: &mut anyhow::Result<_>, chunk| match state {
                                                Err(_) => futures::future::ready(None).boxed(),
                                                Ok((writer, written)) => match chunk {
                                                    Err(e) => {
                                                        futures::future::ready(Some(Err(e))).boxed()
                                                    }
                                                    Ok(chunk) => {
                                                        *written += chunk.len();

                                                        let writer = writer.clone();
                                                        let written = *written;
                                                        async move {
                                                            Some({
                                                                let mut writer =
                                                                    writer.lock().await;
                                                                writer
                                                                    .write_all(&chunk)
                                                                    .map_err(anyhow::Error::from)
                                                                    .map_ok(|_| written)
                                                                    .await
                                                            })
                                                        }
                                                        .boxed()
                                                    }
                                                },
                                            },
                                        )
                                        .boxed()
                                    }
                                },
                                clone!(
                                    #[weak]
                                    obj,
                                    #[upgrade_or]
                                    glib::ControlFlow::Break,
                                    move |result: anyhow::Result<usize>| {
                                        match result {
                                            Ok(written) => {
                                                obj.replace_last_line(&gettext!(
                                                    "Written: {}",
                                                    glib::format_size(written as u64)
                                                ));
                                                glib::ControlFlow::Continue
                                            }
                                            Err(e) => {
                                                obj.insert_line(&e.to_string());
                                                obj.set_state(State::Failed);
                                                glib::ControlFlow::Break
                                            }
                                        }
                                    }
                                ),
                                clone!(
                                    #[weak]
                                    obj,
                                    move || {
                                        obj.insert_line(&gettext("Flushing…"));
                                        utils::do_async(
                                            {
                                                let writer = writer.clone();
                                                async move { writer.lock().await.flush().await }
                                            },
                                            clone!(
                                                #[weak]
                                                obj,
                                                move |result| {
                                                    match result {
                                                        Ok(_) => {
                                                            obj.insert_line(&gettext("Finished"));
                                                            obj.set_state(State::Finished);
                                                        }
                                                        Err(e) => {
                                                            obj.insert_line(&e.to_string());
                                                            obj.set_state(State::Failed);
                                                        }
                                                    }
                                                }
                                            ),
                                        );
                                    }
                                ),
                            );
                        }
                    }
                }
            ),
        );

        obj
    }

    pub(crate) fn pod(num: u32, pod: &str) -> Self {
        Self::new(num, Type::Pod, &gettext!("Create pod <b>{}</b>", pod))
    }

    pub(crate) fn create_pod(
        num: u32,
        pod: &str,
        client: model::Client,
        opts: podman::opts::PodCreateOpts,
    ) -> Self {
        Self::pod(num, pod).create_pod_(client, opts)
    }

    pub(crate) fn create_pod_download_infra(
        num: u32,
        pod: &str,
        client: model::Client,
        pull_opts: podman::opts::PullOpts,
        create_opts_builder: podman::opts::PodCreateOptsBuilder,
    ) -> Self {
        Self::pod(num, pod).download_image_(client, pull_opts, move |obj, client, report| {
            obj.create_pod_(client, create_opts_builder.infra_image(report.id).build());
        })
    }

    pub(crate) fn prune_pods(num: u32, client: model::Client) -> Self {
        let obj = Self::new(num, Type::PrunePods, &gettext("Prune stopped pods"));
        let abort_registration = obj.setup_abort_handle();

        utils::do_async(
            {
                let podman = client.podman();
                async move { stream::Abortable::new(podman.pods().prune(), abort_registration).await }
            },
            clone!(@weak obj => move |result| if let Ok(result) = result {
                let output = obj.output();
                let mut start_iter = output.start_iter();
                match result.as_ref() {
                    Ok(report) => {
                        output.insert(&mut start_iter, &serde_json::to_string_pretty(&report).unwrap());
                        obj.set_state(State::Finished);
                    },
                    Err(e) => {
                        output.insert(&mut start_iter, &e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }),
        );

        obj
    }

    fn setup_abort_handle(&self) -> stream::AbortRegistration {
        let (abort_handle, abort_registration) = stream::AbortHandle::new_pair();
        self.imp().abort_handle.replace(Some(abort_handle));

        abort_registration
    }

    fn download_image_<F>(self, client: model::Client, opts: podman::opts::PullOpts, op: F) -> Self
    where
        F: FnOnce(Self, model::Client, podman::models::LibpodImagesPullReport) + Clone + 'static,
    {
        let abort_registration = self.setup_abort_handle();

        utils::run_stream(
            client.podman().images(),
            move |images| stream::Abortable::new(images.pull(&opts), abort_registration).boxed(),
            clone!(
                @weak self as obj, @weak client => @default-return glib::ControlFlow::Break,
                move |result: podman::Result<podman::models::LibpodImagesPullReport>|
            {
                match result {
                    Ok(report) => match report.error {
                        Some(error) => {
                            log::error!("Error on downloading image: {error}");
                            obj.insert_line(&error);
                            obj.set_state(State::Failed);
                            glib::ControlFlow::Break
                        }
                        None => match report.stream {
                            Some(stream) => {
                                obj.insert(&stream);
                                glib::ControlFlow::Continue
                            }
                            None => {
                                op.clone()(obj, client, report);
                                glib::ControlFlow::Break
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error on downloading image: {e}");
                        obj.insert_line(&e.to_string());
                        obj.set_state(State::Failed);
                        glib::ControlFlow::Break
                    },
                }
            }),
        );

        self
    }

    fn create_container_(
        self,
        client: model::Client,
        opts: podman::opts::ContainerCreateOpts,
        run: bool,
    ) -> Self {
        let abort_registration = self.setup_abort_handle();

        utils::do_async(
            {
                let podman = client.podman();
                async move {
                    stream::Abortable::new(podman.containers().create(&opts), abort_registration)
                        .await
                }
            },
            clone!(@weak self as obj, @weak client => move |result| if let Ok(result) = result {
                match result.map(|info| info.id) {
                    Ok(id) => {
                        match client.container_list().get_container(&id) {
                            Some(container) => {
                                obj.set_artifact(container.upcast_ref());
                                obj.set_state(State::Finished);
                            }
                            None => {
                                client.container_list().connect_container_added(
                                    clone!(@weak obj, @strong id => move |_, container| {
                                        if container.id() == id.as_str() {
                                            obj.set_artifact(container.upcast_ref());
                                            obj.set_state(State::Finished);
                                        }
                                    }),
                                );
                            }
                        }

                        if run {
                            crate::runtime().spawn({
                                let podman = client.podman();
                                async move {
                                    podman
                                        .containers()
                                        .get(id.clone())
                                        .start(None)
                                        .await
                                }
                            });
                        }
                    }
                    Err(e) => {
                        log::error!("Error on creating container: {e}");
                        obj.insert_line(&e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }),
        );

        self
    }

    fn create_pod_(self, client: model::Client, opts: podman::opts::PodCreateOpts) -> Self {
        let abort_registration = self.setup_abort_handle();

        utils::do_async(
            {
                let podman = client.podman();
                async move {
                    stream::Abortable::new(podman.pods().create(&opts), abort_registration).await
                }
            },
            clone!(@weak self as obj, @weak client => move |result| if let Ok(result) = result {
                match result.map(|pod| pod.id().to_string()) {
                    Ok(id) => {
                        match client.pod_list().get_pod(&id) {
                            Some(pod) => {
                                obj.set_artifact(pod.upcast_ref());
                                obj.set_state(State::Finished);
                            },
                            None => {
                                client.pod_list().connect_pod_added(
                                    clone!(@weak obj, @strong id => move |_, pod| {
                                        if pod.id() == id.as_str() {
                                            obj.set_artifact(pod.upcast_ref());
                                            obj.set_state(State::Finished);
                                        }
                                    }),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error on creating pod: {e}");
                        obj.insert(&e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }),
        );

        self
    }

    pub(crate) fn create_volume(
        num: u32,
        name: &str,
        client: model::Client,
        opts: podman::opts::VolumeCreateOpts,
    ) -> Self {
        let obj = Self::new(
            num,
            Type::CreateContainer,
            &gettext!("Create volume <b>{}</b>", name),
        );

        let abort_registration = obj.setup_abort_handle();
        utils::do_async(
            {
                let podman = client.podman();
                async move {
                    stream::Abortable::new(podman.volumes().create(&opts), abort_registration).await
                }
            },
            clone!(@weak obj, @weak client => move |result| if let Ok(result) = result {
                match result.map(|response| response.name.unwrap_or_default()) {
                    Ok(name) => {
                        match client.volume_list().get_volume(&name) {
                            Some(volume) => {
                                obj.set_artifact(volume.upcast_ref());
                                obj.set_state(State::Finished);
                            }
                            None => {
                                client.volume_list().connect_volume_added(
                                    clone!(@weak obj, @strong name => move |_, volume| {
                                        if volume.inner().name == name {
                                            obj.set_artifact(volume.upcast_ref());
                                            obj.set_state(State::Finished);
                                        }
                                    }),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error on creating volume: {e}");
                        obj.insert_line(&e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }),
        );

        obj
    }

    pub(crate) fn prune_volumes(
        num: u32,
        client: model::Client,
        opts: podman::opts::VolumePruneOpts,
    ) -> Self {
        let obj = Self::new(num, Type::PruneVolumes, &gettext("Prune unused volumes"));
        let abort_registration = obj.setup_abort_handle();

        utils::do_async(
            {
                let podman = client.podman();
                async move {
                    stream::Abortable::new(podman.volumes().prune(&opts), abort_registration).await
                }
            },
            clone!(@weak obj => move |result| if let Ok(result) = result {
                let output = obj.output();
                let mut start_iter = output.start_iter();
                match result.as_ref() {
                    Ok(report) => {
                        output.insert(&mut start_iter, &serde_json::to_string_pretty(&report).unwrap());
                        obj.set_state(State::Finished);
                    },
                    Err(e) => {
                        output.insert(&mut start_iter, &e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }),
        );

        obj
    }
}

impl Action {
    fn set_artifact(&self, value: &glib::Object) {
        if self.artifact().is_some() {
            return;
        }

        self.insert_line(&gettext("Finished"));

        self.imp().artifact.set(Some(value));
        self.notify_artifact();
    }

    fn set_state(&self, value: State) {
        if self.state() == value {
            return;
        }

        if value != State::Ongoing {
            self.set_end_timestamp(glib::DateTime::now_local().unwrap().to_unix());
        }

        self.imp().state.set(value);
        self.notify_state();
    }

    fn set_end_timestamp(&self, value: i64) {
        let imp = self.imp();

        if imp.end_timestamp.get().is_some() {
            return;
        }
        imp.end_timestamp.set(value).unwrap();
        self.notify_end_timestamp();
    }

    fn insert(&self, text: &str) {
        let output = self.output();
        let mut iter = output.start_iter();

        output.insert(&mut iter, text);
    }

    fn insert_line(&self, text: &str) {
        self.insert(&format!("{text}\n"));
    }

    fn replace_last_line(&self, text: &str) {
        let output = self.output();

        let mut start_iter = output.start_iter();
        let mut end_iter = output.start_iter();
        end_iter.forward_line();

        output.delete(&mut start_iter, &mut end_iter);
        output.insert(&mut start_iter, &format!("{text}\n"));
    }
}
