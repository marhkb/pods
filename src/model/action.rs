use std::cell::Cell;
use std::cell::RefCell;

use futures::stream;
use futures::StreamExt;
use gettextrs::gettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

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
    Cancelled,
    Failed,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ActionType")]
pub(crate) enum Type {
    PruneImages,
    DownloadImage,
    BuildImage,
    Container,
    Pod,
    #[default]
    Undefined,
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct Action {
        pub(super) abort_handle: RefCell<Option<stream::AbortHandle>>,
        pub(super) artifact: glib::WeakRef<glib::Object>,
        pub(super) num: OnceCell<u32>,
        pub(super) type_: OnceCell<Type>,
        pub(super) name: OnceCell<String>,
        pub(super) state: Cell<State>,
        pub(super) start_timestamp: OnceCell<i64>,
        pub(super) end_timestamp: OnceCell<i64>,
        pub(super) output: gtk::TextBuffer,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Action {
        const NAME: &'static str = "Action";
        type Type = super::Action;
    }

    impl ObjectImpl for Action {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "artifact",
                        "Artifact",
                        "The resulting artifact of the action",
                        glib::Object::static_type(),
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt::new(
                        "num",
                        "Num",
                        "The action number",
                        0,
                        u32::MAX,
                        0,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecEnum::new(
                        "type",
                        "Type",
                        "The type of the action",
                        Type::static_type(),
                        Type::default() as i32,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "name",
                        "Name",
                        "The name of the action",
                        None,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecEnum::new(
                        "state",
                        "State",
                        "The current state of the action",
                        State::static_type(),
                        State::default() as i32,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecInt64::new(
                        "start-timestamp",
                        "Start Timestamp",
                        "The timestamp when the action started",
                        i64::MIN,
                        i64::MAX,
                        0,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecInt64::new(
                        "end-timestamp",
                        "End Timestamp",
                        "The timestamp when the action ended",
                        i64::MIN,
                        i64::MAX,
                        0,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecObject::new(
                        "output",
                        "Output",
                        "The output of the action",
                        gtk::TextBuffer::static_type(),
                        glib::ParamFlags::READABLE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "num" => self.num.set(value.get().unwrap()).unwrap(),
                "type" => self.type_.set(value.get().unwrap()).unwrap(),
                "name" => self.name.set(value.get().unwrap()).unwrap(),
                "start-timestamp" => self.start_timestamp.set(value.get().unwrap()).unwrap(),
                "end-timestamp" => self.end_timestamp.set(value.get().unwrap()).unwrap(),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "num" => obj.num().to_value(),
                "type" => obj.type_().to_value(),
                "name" => obj.name().to_value(),
                "state" => obj.state().to_value(),
                "start-timestamp" => obj.start_timestamp().to_value(),
                "end-timestamp" => obj.end_timestamp().to_value(),
                "output" => obj.output().to_value(),
                _ => unimplemented!(),
            }
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
            self.set_state(State::Cancelled);
        }
    }
}

impl Action {
    fn new(num: u32, type_: Type, name: &str) -> Self {
        glib::Object::new(&[
            ("num", &num),
            ("type", &type_),
            ("name", &name),
            (
                "start-timestamp",
                &glib::DateTime::now_local().unwrap().to_unix(),
            ),
        ])
        .expect("Failed to create Action")
    }

    pub(crate) fn prune_images(
        num: u32,
        client: model::Client,
        opts: podman::opts::ImagePruneOpts,
    ) -> Self {
        let obj = Self::new(num, Type::PruneImages, "");
        let abort_registration = obj.setup_abort_handle();

        utils::do_async(
            {
                let podman = client.podman().clone();
                async move {
                    stream::Abortable::new(podman.images().prune(&opts), abort_registration).await
                }
            },
            clone!(@weak obj => move |result| if let Ok(result) = result {
                let ouput = obj.output();
                let mut start_iter = ouput.start_iter();
                match result.as_ref() {
                    Ok(report) => {
                        ouput.insert(&mut start_iter, &serde_json::to_string_pretty(&report).unwrap());
                        obj.set_state(State::Finished);
                    },
                    Err(e) => {
                        ouput.insert(&mut start_iter, &e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }),
        );

        obj
    }

    pub(crate) fn download_image(
        num: u32,
        name: &str,
        client: model::Client,
        opts: podman::opts::PullOpts,
    ) -> Self {
        Self::new(num, Type::DownloadImage, name).download_image_(
            client,
            opts,
            |obj, client, report| {
                let image_id = report.id.unwrap();
                match client.image_list().get_image(&image_id) {
                    Some(image) => {
                        obj.set_artifact(image.upcast_ref());
                        obj.set_state(State::Finished);
                    }
                    None => {
                        client.image_list().connect_image_added(
                            clone!(@weak obj => move |_, image| {
                                if image.id() == image_id.as_str() {
                                    obj.set_artifact(image.upcast_ref());
                                    obj.set_state(State::Finished);
                                }
                            }),
                        );
                    }
                }
            },
        )
    }

    pub(crate) fn build_image(
        num: u32,
        name: &str,
        client: model::Client,
        opts: podman::opts::ImageBuildOpts,
    ) -> Self {
        let obj = Self::new(num, Type::BuildImage, name);
        let abort_registration = obj.setup_abort_handle();

        obj.insert_text(&gettext("Generating tarball of context directory..."));

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
                @weak obj, @weak client => @default-return glib::Continue(false),
                move |result: podman::Result<podman::models::ImageBuildLibpod200Response>|
            {
                glib::Continue(match result {
                    Ok(stream) => {
                        obj.insert_text(&stream.stream);
                        true
                    }
                    Err(e) => {
                        log::error!("Error on building image: {e}");
                        obj.insert_text(&e.to_string());
                        obj.set_state(State::Failed);
                        false
                    },
                })
            }),
            clone!(@weak obj, @weak client => @default-return glib::Continue(false), move |_| {
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

                glib::Continue(false)
            }),
        );

        obj
    }

    pub(crate) fn create_container(
        num: u32,
        name: &str,
        client: model::Client,
        opts: podman::opts::ContainerCreateOpts,
        run: bool,
    ) -> Self {
        Self::new(num, Type::Container, name).create_container_(client, opts, run)
    }

    pub(crate) fn create_container_download_image(
        num: u32,
        name: &str,
        client: model::Client,
        pull_opts: podman::opts::PullOpts,
        create_opts_builder: podman::opts::ContainerCreateOptsBuilder,
        run: bool,
    ) -> Self {
        Self::new(num, Type::Container, name).download_image_(
            client,
            pull_opts,
            move |obj, client, report| {
                obj.create_container_(client, create_opts_builder.image(report.id).build(), run);
            },
        )
    }

    pub(crate) fn create_pod(
        num: u32,
        name: &str,
        client: model::Client,
        opts: podman::opts::PodCreateOpts,
    ) -> Self {
        Self::new(num, Type::Pod, name).create_pod_(client, opts)
    }

    pub(crate) fn create_pod_download_infra(
        num: u32,
        name: &str,
        client: model::Client,
        pull_opts: podman::opts::PullOpts,
        create_opts_builder: podman::opts::PodCreateOptsBuilder,
    ) -> Self {
        Self::new(num, Type::Pod, name).download_image_(
            client,
            pull_opts,
            move |obj, client, report| {
                obj.create_pod_(client, create_opts_builder.infra_image(report.id).build());
            },
        )
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
                @weak self as obj, @weak client => @default-return glib::Continue(false),
                move |result: podman::Result<podman::models::LibpodImagesPullReport>|
            {
                glib::Continue(match result {
                    Ok(report) => match report.error {
                        Some(error) => {
                            log::error!("Error on downloading image: {error}");
                            obj.insert_text(&error);
                            obj.set_state(State::Failed);
                            false
                        }
                        None => match report.stream {
                            Some(stream) => {
                                obj.insert_text(&stream);
                                true
                            }
                            None => {
                                op.clone()(obj, client, report);
                                false
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error on downloading image: {e}");
                        obj.insert_text(&e.to_string());
                        obj.set_state(State::Failed);
                        false
                    },
                })
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
                let podman = client.podman().clone();
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
                            crate::RUNTIME.spawn({
                                let podman = client.podman().clone();
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
                        obj.insert_text(&e.to_string());
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
                let podman = client.podman().clone();
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
                        obj.insert_text(&e.to_string());
                        obj.set_state(State::Failed);
                    }
                }
            }),
        );

        self
    }
}

impl Action {
    pub(crate) fn artifact(&self) -> Option<glib::Object> {
        self.imp().artifact.upgrade()
    }

    fn set_artifact(&self, value: &glib::Object) {
        if self.artifact().is_some() {
            return;
        }

        self.insert_text(&gettext("Finished\n"));

        self.imp().artifact.set(Some(value));
        self.notify("artifact");
    }

    pub(crate) fn num(&self) -> u32 {
        *self.imp().num.get().unwrap()
    }

    pub(crate) fn type_(&self) -> Type {
        *self.imp().type_.get().unwrap()
    }

    pub(crate) fn name(&self) -> &str {
        self.imp().name.get().unwrap()
    }

    pub(crate) fn state(&self) -> State {
        self.imp().state.get()
    }

    fn set_state(&self, value: State) {
        if self.state() == value {
            return;
        }

        if value != State::Ongoing {
            self.set_end_timesamp(glib::DateTime::now_local().unwrap().to_unix());
        }

        self.imp().state.set(value);
        self.notify("state");
    }

    pub(crate) fn start_timestamp(&self) -> i64 {
        *self.imp().start_timestamp.get().unwrap()
    }

    pub(crate) fn end_timestamp(&self) -> i64 {
        *self.imp().end_timestamp.get().unwrap_or(&0)
    }

    fn set_end_timesamp(&self, value: i64) {
        let imp = self.imp();

        if imp.end_timestamp.get().is_some() {
            return;
        }
        imp.end_timestamp.set(value).unwrap();
        self.notify("end-timestamp");
    }

    pub(crate) fn output(&self) -> gtk::TextBuffer {
        self.imp().output.clone()
    }

    fn insert_text(&self, text: &str) {
        let output = self.output();
        let mut iter = output.start_iter();

        output.insert(&mut iter, text);
    }
}
