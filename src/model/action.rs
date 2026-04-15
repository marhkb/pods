use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::Arc;

use adw::prelude::*;
use futures::FutureExt;
use futures::StreamExt;
use futures::TryFutureExt;
use futures::future;
use futures::lock::Mutex;
use futures::stream;
use gettextrs::gettext;
use gio::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use gtk::gio;
use gtk::glib;
use tokio::io::AsyncWriteExt;
use tokio::io::BufWriter;

use crate::engine;
use crate::model;
use crate::model::prelude::*;
use crate::rt;

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
        opts: engine::opts::ImagesPruneOpts,
    ) -> Self {
        let obj = Self::new(num, Type::PruneImages, &gettext("Prune unused images"));
        let abort_registration = obj.setup_abort_handle();

        rt::Promise::new({
            let api = client.engine().images();
            async move { future::Abortable::new(api.prune(opts), abort_registration).await }
        })
        .defer(clone!(
            #[weak]
            obj,
            move |result| if let Ok(result) = result {
                let output = obj.output();
                let mut start_iter = output.start_iter();
                match result.as_ref() {
                    Ok(report) => {
                        output.insert(&mut start_iter, report);
                        obj.set_state(State::Finished);
                    }
                    Err(e) => {
                        output.insert(&mut start_iter, &e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }
        ));

        obj
    }

    pub(crate) fn download_image(
        num: u32,
        client: model::Client,
        opts: engine::opts::ImagePullOpts,
    ) -> Self {
        Self::new(
            num,
            Type::DownloadImage,
            &gettext!("Pull image <b>{}</b>", opts.reference),
        )
        .download_image_(client, opts, |obj, client, image_id| {
            match client.image_list().get_image(&image_id) {
                Some(image) => {
                    obj.set_artifact(image.upcast_ref());
                    obj.set_state(State::Finished);
                }
                None => {
                    client.image_list().connect_image_added(clone!(
                        #[weak]
                        obj,
                        move |_, image| {
                            if image.id() == image_id.as_str() {
                                obj.set_artifact(image.upcast_ref());
                                obj.set_state(State::Finished);
                            }
                        }
                    ));
                }
            }
        })
    }

    pub(crate) fn push_image(
        num: u32,
        api: engine::api::Image,
        repo: String,
        opts: engine::opts::ImagePushOpts,
        credentials: Option<engine::auth::Credentials>,
    ) -> Self {
        let obj = Self::new(
            num,
            Type::PushImage,
            &gettext!("Push image <b>{}</b>", format!("{repo}:{}", opts.tag)),
        );
        let abort_registration = obj.setup_abort_handle();

        rt::Pipe::new(api, move |api| {
            future::Abortable::new(api.push(repo, opts, credentials), abort_registration).boxed()
        })
        .on_next(clone!(
            #[weak]
            obj,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move |report| {
                match report {
                    Ok(report) => match report.stream {
                        Some(line) => {
                            obj.insert(&line);
                            glib::ControlFlow::Continue
                        }
                        None => match report.error {
                            Some(error) => {
                                log::error!("Error on pushing image: {error}");
                                obj.insert(&error);
                                obj.set_state(State::Failed);
                                glib::ControlFlow::Break
                            }
                            None => glib::ControlFlow::Continue,
                        },
                    },
                    Err(e) => {
                        log::error!("Error on pushing image: {e}");
                        obj.insert_line(&e.to_string());
                        obj.set_state(State::Failed);
                        glib::ControlFlow::Break
                    }
                }
            }
        ))
        .on_finish(clone!(
            #[weak]
            obj,
            move || {
                if obj.state() != State::Failed {
                    obj.set_state(State::Finished);
                }
            }
        ));

        obj
    }

    pub(crate) async fn build_image(
        num: u32,
        client: model::Client,
        opts: engine::opts::ImageBuildOpts,
        context_dir: String,
    ) -> Self {
        let obj = Self::new(
            num,
            Type::BuildImage,
            &gettext!(
                "Build image <b>{}</b>",
                opts.tag.as_deref().unwrap_or_default()
            ),
        );
        let abort_registration = obj.setup_abort_handle();

        obj.insert_line(&gettext("Generating tarball of context directory..."));

        rt::Pipe::new(client.engine().images(), move |images| {
            match images.build(opts, context_dir) {
                Ok(stream) => stream::Abortable::new(stream, abort_registration).boxed(),
                Err(e) => {
                    log::error!("Error on building image: {e}");
                    futures::stream::empty().boxed()
                }
            }
        })
        .on_next(clone!(
            #[weak]
            obj,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move |report| {
                match report {
                    Ok(report) => match report {
                        engine::dto::ImageBuildReport::Error { message } => {
                            obj.insert(&message);
                            obj.set_state(State::Failed);
                            glib::ControlFlow::Break
                        }
                        engine::dto::ImageBuildReport::Streaming { line } => {
                            obj.insert(&line);
                            glib::ControlFlow::Continue
                        }
                        engine::dto::ImageBuildReport::Finished { image_id } => {
                            match client.image_list().get_image(&image_id) {
                                Some(image) => {
                                    obj.set_artifact(image.upcast_ref());
                                    obj.set_state(State::Finished);
                                }
                                None => {
                                    client.image_list().connect_image_added(clone!(
                                        #[weak]
                                        obj,
                                        move |_, image| {
                                            if image.id() == image_id {
                                                obj.set_artifact(image.upcast_ref());
                                                obj.set_state(State::Finished);
                                            }
                                        }
                                    ));
                                }
                            }

                            glib::ControlFlow::Break
                        }
                    },
                    Err(e) => {
                        log::error!("Error on building image: {e}");
                        obj.insert_line(&e.to_string());
                        obj.set_state(State::Failed);
                        glib::ControlFlow::Break
                    }
                }
            }
        ));

        obj
    }

    pub(crate) fn prune_containers(num: u32, client: model::Client, until: Option<String>) -> Self {
        let obj = Self::new(
            num,
            Type::PruneContainers,
            &gettext("Prune stopped containers"),
        );
        let abort_registration = obj.setup_abort_handle();

        rt::Promise::new({
            let engine = client.engine().inner();
            async move {
                future::Abortable::new(engine.containers().prune(until), abort_registration).await
            }
        })
        .defer(clone!(
            #[weak]
            obj,
            move |report| if let Ok(report) = report {
                let output = obj.output();
                let mut start_iter = output.start_iter();
                match report {
                    Ok(report) => {
                        output.insert(&mut start_iter, &report);
                        obj.set_state(State::Finished);
                    }
                    Err(e) => {
                        output.insert(&mut start_iter, &e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }
        ));

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
        client: model::Client,
        opts: engine::opts::ContainerCreateOpts,
        run: bool,
    ) -> Self {
        Self::container(num, &opts.name, run).create_container_(client, opts, run)
    }

    pub(crate) fn commit_container(
        num: u32,
        container: &str,
        api: engine::api::Container,
        opts: engine::opts::ContainerCommitOpts,
    ) -> Self {
        let image = opts.repo.as_ref().map(|repo| {
            format!(
                "{}:{}",
                repo,
                opts.tag.clone().unwrap_or_else(|| "latest".to_string())
            )
        });

        let obj = Self::new(
            num,
            Type::Commit,
            &gettext!(
                "Commit image <b>{}</b> ({})",
                container,
                image.unwrap_or_else(|| format!("<i>&lt;{}&gt;</i>", gettext("none"))),
            ),
        );

        let abort_registration = obj.setup_abort_handle();

        rt::Promise::new(async move {
            future::Abortable::new(api.commit(opts), abort_registration).await
        })
        .defer(clone!(
            #[weak]
            obj,
            move |result| match result.as_ref() {
                Ok(_) => {
                    obj.insert_line(&gettext("Finished"));
                    obj.set_state(State::Finished);
                }
                Err(e) => {
                    obj.insert_line(&e.to_string());
                    obj.set_state(State::Failed);
                }
            }
        ));

        obj
    }

    pub(crate) fn create_container_download_image(
        num: u32,
        client: model::Client,
        image_pull_opts: engine::opts::ImagePullOpts,
        container_create_opts: engine::opts::ContainerCreateOpts,
        run: bool,
    ) -> Self {
        Self::container(num, &container_create_opts.name, run).download_image_(
            client,
            image_pull_opts,
            move |obj, client, _image_id| {
                obj.create_container_(client, container_create_opts, run);
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

        rt::Promise::new(async move {
            let mut ar = tokio_tar::Builder::new(Vec::new());

            future::Abortable::new(
                async move {
                    let host_path = PathBuf::from(host_path);
                    let file_name = host_path.file_name();
                    if directory {
                        ar.append_dir_all(file_name.unwrap_or_else(|| OsStr::new(".")), &host_path)
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
        })
        .defer(clone!(
            #[weak]
            obj,
            #[weak]
            container,
            move |result| if let Ok(result) = result {
                match result {
                    Ok(ar) => {
                        obj.insert_line(&gettext("Tar archive created"));
                        obj.insert_line(&gettext("Unwrapping tar archive…"));

                        let abort_registration = obj.setup_abort_handle();
                        rt::Promise::new(future::Abortable::new(
                            ar.into_inner(),
                            abort_registration,
                        ))
                        .defer(clone!(
                            #[weak]
                            obj,
                            #[weak]
                            container,
                            move |result| if let Ok(result) = result {
                                match result {
                                    Ok(buf) => {
                                        obj.insert_line(&gettext("Tar archive unwrapped"));
                                        obj.insert_line(&gettext("Copying files into container…"));

                                        let abort_registration = obj.setup_abort_handle();
                                        let api = container.api().unwrap();
                                        rt::Promise::new(async move {
                                            stream::Abortable::new(
                                                api.copy_to(container_path, buf),
                                                abort_registration,
                                            )
                                            .await
                                        })
                                        .defer(clone!(
                                            #[weak]
                                            obj,
                                            move |result| match result {
                                                Ok(_) => {
                                                    obj.insert_line(&gettext("Finished"));
                                                    obj.set_state(State::Finished);
                                                }
                                                Err(e) => {
                                                    obj.insert_line(&e.to_string());
                                                    obj.set_state(State::Failed);
                                                }
                                            }
                                        ));
                                    }
                                    Err(e) => {
                                        obj.insert_line(&e.to_string());
                                        obj.set_state(State::Failed);
                                    }
                                }
                            }
                        ));
                    }
                    Err(e) => {
                        obj.insert_line(&e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }
        ));

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

        rt::Promise::new(async move {
            tokio::fs::File::options()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&host_path)
                .await
        })
        .defer(clone!(
            #[weak]
            obj,
            #[weak]
            container,
            move |result| match result {
                Err(e) => {
                    obj.insert_line(&e.to_string());
                    obj.set_state(State::Failed);
                }
                Ok(file) => {
                    let writer = Arc::new(Mutex::new(BufWriter::new(file)));
                    obj.insert_line(&gettext!("Written: {}", glib::format_size(0)));

                    rt::Pipe::new(container.api().unwrap(), {
                        let writer = writer.clone();
                        move |container| {
                            stream::Abortable::new(
                                container.copy_from(container_path),
                                abort_registration,
                            )
                            .scan(Ok((writer, 0)), |state: &mut anyhow::Result<_>, chunk| {
                                match state {
                                    Err(_) => future::ready(None).boxed(),
                                    Ok((writer, written)) => match chunk {
                                        Err(e) => future::ready(Some(Err(e))).boxed(),
                                        Ok(chunk) => {
                                            *written += chunk.len();

                                            let writer = writer.clone();
                                            let written = *written;
                                            async move {
                                                Some({
                                                    let mut writer = writer.lock().await;
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
                                }
                            })
                            .boxed()
                        }
                    })
                    .on_next(clone!(
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
                    ))
                    .on_finish(clone!(
                        #[weak]
                        obj,
                        move || {
                            obj.insert_line(&gettext("Flushing…"));
                            rt::Promise::new({
                                let writer = writer.clone();
                                async move { writer.lock().await.flush().await }
                            })
                            .defer(clone!(
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
                            ));
                        }
                    ));
                }
            }
        ));

        obj
    }

    pub(crate) fn pod(num: u32, pod: &str) -> Self {
        Self::new(num, Type::Pod, &gettext!("Create pod <b>{}</b>", pod))
    }

    pub(crate) fn create_pod(
        num: u32,
        client: model::Client,
        opts: engine::opts::PodCreateOpts,
    ) -> Option<Self> {
        Self::pod(num, &opts.name).create_pod_(client, opts)
    }

    pub(crate) fn create_pod_download_infra(
        num: u32,
        client: model::Client,
        image_pull_opts: engine::opts::ImagePullOpts,
        pod_create_opts: engine::opts::PodCreateOpts,
    ) -> Self {
        Self::pod(num, &pod_create_opts.name).download_image_(
            client,
            image_pull_opts,
            |obj, client, _image_id| {
                obj.create_pod_(client, pod_create_opts);
            },
        )
    }

    pub(crate) fn prune_pods(num: u32, api: engine::api::Pods) -> Self {
        let obj = Self::new(num, Type::PrunePods, &gettext("Prune stopped pods"));
        let abort_registration = obj.setup_abort_handle();

        rt::Promise::new(
            async move { future::Abortable::new(api.prune(), abort_registration).await },
        )
        .defer(clone!(
            #[weak]
            obj,
            move |report| if let Ok(report) = report {
                let output = obj.output();
                let mut start_iter = output.start_iter();
                match report.as_ref() {
                    Ok(report) => {
                        output.insert(
                            &mut start_iter,
                            &serde_json::to_string_pretty(&report).unwrap(),
                        );
                        obj.set_state(State::Finished);
                    }
                    Err(e) => {
                        output.insert(&mut start_iter, &e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }
        ));

        obj
    }

    fn setup_abort_handle(&self) -> stream::AbortRegistration {
        let (abort_handle, abort_registration) = stream::AbortHandle::new_pair();
        self.imp().abort_handle.replace(Some(abort_handle));

        abort_registration
    }

    fn download_image_<F>(
        self,
        client: model::Client,
        opts: engine::opts::ImagePullOpts,
        op: F,
    ) -> Self
    where
        F: FnOnce(Self, model::Client, String) + Clone + 'static,
    {
        let abort_registration = self.setup_abort_handle();

        rt::Pipe::new(client.engine().images(), move |images| {
            stream::Abortable::new(images.pull(opts), abort_registration).boxed()
        })
        .on_next(clone!(
            #[weak(rename_to = obj)]
            self,
            #[weak]
            client,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move |report| match report {
                Ok(report) => match report {
                    engine::dto::ImagePullReport::Error { message } => {
                        log::error!("Error on downloading image: {message}");
                        obj.insert_line(&message);
                        obj.set_state(State::Failed);
                        glib::ControlFlow::Break
                    }
                    engine::dto::ImagePullReport::Streaming { line } => {
                        obj.insert(&line);
                        glib::ControlFlow::Continue
                    }
                    engine::dto::ImagePullReport::Finished { image_id } => {
                        obj.set_state(State::Finished);
                        op.clone()(obj, client, image_id);
                        glib::ControlFlow::Break
                    }
                },
                Err(e) => {
                    log::error!("Error on downloading image: {e}");
                    obj.insert_line(&e.to_string());
                    obj.set_state(State::Failed);
                    glib::ControlFlow::Break
                }
            }
        ));

        self
    }

    fn create_container_(
        self,
        client: model::Client,
        opts: engine::opts::ContainerCreateOpts,
        run: bool,
    ) -> Self {
        let abort_registration = self.setup_abort_handle();

        rt::Promise::new({
            let engine = client.engine().inner();
            async move {
                future::Abortable::new(engine.containers().create(opts), abort_registration).await
            }
        })
        .defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |id| if let Ok(id) = id {
                match id {
                    Ok(id) => {
                        match client.container_list().get_container(&id) {
                            Some(container) => {
                                obj.set_artifact(container.upcast_ref());
                                obj.set_state(State::Finished);
                            }
                            None => {
                                client.container_list().connect_container_added(clone!(
                                    #[weak]
                                    obj,
                                    #[strong]
                                    id,
                                    move |_, container| {
                                        if container.id() == id.as_str() {
                                            obj.set_artifact(container.upcast_ref());
                                            obj.set_state(State::Finished);
                                        }
                                    }
                                ));
                            }
                        }

                        if run {
                            rt::Promise::new({
                                let engine = client.engine().inner();
                                async move { engine.containers().get(id).start().await }
                            })
                            .spawn();
                        }
                    }
                    Err(e) => {
                        log::error!("Error on creating container: {e}");
                        obj.insert_line(&e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }
        ));

        self
    }

    fn create_pod_(self, client: model::Client, opts: engine::opts::PodCreateOpts) -> Option<Self> {
        let pod_list = client.pod_list()?;

        let abort_registration = self.setup_abort_handle();

        rt::Promise::new({
            let engine = client.engine().inner();
            async move { stream::Abortable::new(engine.pods().create(opts), abort_registration).await }
        })
        .defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |id| if let Ok(id) = id {
                match id {
                    Ok(id) => match pod_list.get_pod(&id) {
                        Some(pod) => {
                            obj.set_artifact(pod.upcast_ref());
                            obj.set_state(State::Finished);
                        }
                        None => {
                            pod_list.connect_pod_added(clone!(
                                #[weak]
                                obj,
                                #[strong]
                                id,
                                move |_, pod| {
                                    if pod.id() == id.as_str() {
                                        obj.set_artifact(pod.upcast_ref());
                                        obj.set_state(State::Finished);
                                    }
                                }
                            ));
                        }
                    },
                    Err(e) => {
                        log::error!("Error on creating pod: {e}");
                        obj.insert(&e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }
        ));

        Some(self)
    }

    pub(crate) fn create_volume(num: u32, name: String, client: model::Client) -> Self {
        let obj = Self::new(
            num,
            Type::Volume,
            &gettext!("Create volume <b>{}</b>", name),
        );

        let abort_registration = obj.setup_abort_handle();

        rt::Promise::new({
            let engine = client.engine().inner();
            async move {
                future::Abortable::new(engine.volumes().create(name), abort_registration).await
            }
        })
        .defer(clone!(
            #[weak]
            obj,
            move |name| if let Ok(name) = name {
                match name {
                    Ok(name) => match client.volume_list().get_volume(&name) {
                        Some(volume) => {
                            obj.set_artifact(volume.upcast_ref());
                            obj.set_state(State::Finished);
                        }
                        None => {
                            client.volume_list().connect_volume_added(clone!(
                                #[weak]
                                obj,
                                #[strong]
                                name,
                                move |_, volume| if volume.name() == name {
                                    obj.set_artifact(volume.upcast_ref());
                                    obj.set_state(State::Finished);
                                }
                            ));
                        }
                    },
                    Err(e) => {
                        log::error!("Error on creating volume: {e}");
                        obj.insert_line(&e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }
        ));

        obj
    }

    pub(crate) fn prune_volumes(
        num: u32,
        client: model::Client,
        opts: engine::opts::VolumesPruneOpts,
    ) -> Self {
        let obj = Self::new(num, Type::PruneVolumes, &gettext("Prune unused volumes"));
        let abort_registration = obj.setup_abort_handle();

        rt::Promise::new({
            let engine = client.engine().inner();
            async move {
                future::Abortable::new(engine.volumes().prune(opts), abort_registration).await
            }
        })
        .defer(clone!(
            #[weak]
            obj,
            move |result| if let Ok(report) = result {
                let output = obj.output();
                let mut start_iter = output.start_iter();
                match report {
                    Ok(report) => {
                        output.insert(&mut start_iter, &report);
                        obj.set_state(State::Finished);
                    }
                    Err(e) => {
                        output.insert(&mut start_iter, &e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }
        ));

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
