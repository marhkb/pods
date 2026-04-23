use std::cell::Cell;
use std::cell::RefCell;
use std::marker::PhantomData;

use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use gtk::gio;
use gtk::glib;
use indexmap::IndexMap;

use crate::engine;
use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ActionList)]
    pub(crate) struct ActionList {
        pub(super) list: RefCell<IndexMap<u32, model::Action>>,
        pub(super) action_counter: Cell<u32>,

        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,

        #[property(get = Self::len)]
        _len: PhantomData<u32>,
        #[property(get = Self::ongoing)]
        _ongoing: PhantomData<u32>,
        #[property(get = Self::finished)]
        _finished: PhantomData<u32>,
        #[property(get = Self::aborted)]
        _aborted: PhantomData<u32>,
        #[property(get = Self::failed)]
        _failed: PhantomData<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ActionList {
        const NAME: &'static str = "ActionList";
        type Type = super::ActionList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for ActionList {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().connect_items_changed(|obj, _, _, _| {
                obj.notify_len();
                obj.notify_num_states();
            });
        }
    }

    impl ListModelImpl for ActionList {
        fn item_type(&self) -> glib::Type {
            model::Action::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .map(|(_, obj)| obj.upcast_ref())
                .cloned()
        }
    }

    impl ActionList {
        fn len(&self) -> u32 {
            self.n_items()
        }

        fn ongoing(&self) -> u32 {
            self.count_state(model::ActionState::Ongoing)
        }

        fn finished(&self) -> u32 {
            self.count_state(model::ActionState::Finished)
        }

        fn aborted(&self) -> u32 {
            self.count_state(model::ActionState::Aborted)
        }

        fn failed(&self) -> u32 {
            self.count_state(model::ActionState::Failed)
        }

        fn count_state(&self, state: model::ActionState) -> u32 {
            self.list
                .borrow()
                .values()
                .filter(|action| action.state() == state)
                .count() as u32
        }
    }
}

glib::wrapper! {
    pub(crate) struct ActionList(ObjectSubclass<imp::ActionList>)
        @implements gio::ListModel;
}

impl From<&model::Client> for ActionList {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl ActionList {
    pub(crate) fn download_image(&self, opts: engine::opts::ImagePullOpts) -> model::Action {
        self.insert_action(model::Action::download_image(
            self.imp().action_counter.get(),
            self.client().unwrap(),
            opts,
        ))
    }

    pub(crate) fn push_image(
        &self,
        api: engine::api::Image,
        repo: String,
        opts: engine::opts::ImagePushOpts,
        credentials: Option<engine::auth::Credentials>,
    ) -> model::Action {
        self.insert_action(model::Action::push_image(
            self.imp().action_counter.get(),
            api,
            repo,
            opts,
            credentials,
        ))
    }

    pub(crate) async fn build_image(
        &self,
        opts: engine::opts::ImageBuildOpts,
        context_dir: String,
    ) -> model::Action {
        self.insert_action(
            model::Action::build_image(
                self.imp().action_counter.get(),
                self.client().unwrap(),
                opts,
                context_dir,
            )
            .await,
        )
    }

    pub(crate) fn commit_container(
        &self,
        container: &str,
        api: engine::api::Container,
        opts: engine::opts::ContainerCommitOpts,
    ) -> model::Action {
        self.insert_action(model::Action::commit_container(
            self.imp().action_counter.get(),
            container,
            api,
            opts,
        ))
    }

    pub(crate) fn copy_files_into_container(
        &self,
        host_path: String,
        container_path: String,
        directory: bool,
        container: &model::Container,
    ) -> model::Action {
        self.insert_action(model::Action::copy_files_into_container(
            self.imp().action_counter.get(),
            host_path,
            container_path,
            directory,
            container,
        ))
    }

    pub(crate) fn copy_files_from_container(
        &self,
        container: &model::Container,
        container_path: String,
        host_path: String,
    ) -> model::Action {
        self.insert_action(model::Action::copy_files_from_container(
            self.imp().action_counter.get(),
            container,
            container_path,
            host_path,
        ))
    }

    pub(crate) fn create_pod(&self, opts: engine::opts::PodCreateOpts) -> Option<model::Action> {
        model::Action::create_pod(
            self.imp().action_counter.get(),
            self.client().unwrap(),
            opts,
        )
        .map(|action| self.insert_action(action))
    }

    pub(crate) fn create_pod_download_infra(
        &self,
        image_pull_opts: engine::opts::ImagePullOpts,
        pod_create_opts: engine::opts::PodCreateOpts,
    ) -> model::Action {
        self.insert_action(model::Action::create_pod_download_infra(
            self.imp().action_counter.get(),
            self.client().unwrap(),
            image_pull_opts,
            pod_create_opts,
        ))
    }

    pub(crate) fn create_volume(&self, name: String) -> model::Action {
        self.insert_action(model::Action::create_volume(
            self.imp().action_counter.get(),
            name,
            self.client().unwrap(),
        ))
    }

    fn insert_action(&self, action: model::Action) -> model::Action {
        let imp = self.imp();

        let position = {
            let mut list = imp.list.borrow_mut();
            list.insert(imp.action_counter.replace(action.num() + 1), action.clone());
            list.len() - 1
        };

        action.connect_notify_local(
            Some("state"),
            clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, _| obj.notify_num_states()
            ),
        );

        self.items_changed(position as u32, 0, 1);

        action
    }

    fn notify_num_states(&self) {
        self.notify_ongoing();
        self.notify_finished();
        self.notify_aborted();
        self.notify_failed();
    }
}
