use std::cell::RefCell;
use std::marker::PhantomData;

use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use gtk::gio;
use gtk::glib;
use indexmap::IndexSet;

use crate::engine;
use crate::model;
use crate::model::ActionExt;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ActionList)]
    pub(crate) struct ActionList {
        pub(super) list: RefCell<IndexSet<model::Action>>,

        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,

        #[property(get = Self::len)]
        _len: PhantomData<u32>,
        #[property(get = Self::ongoing)]
        _ongoing: PhantomData<u32>,
        #[property(get = Self::finished)]
        _finished: PhantomData<u32>,
        #[property(get = Self::cancelled)]
        _cancelled: PhantomData<u32>,
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
                .map(model::Action::upcast_ref)
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

        fn cancelled(&self) -> u32 {
            self.count_state(model::ActionState::Cancelled)
        }

        fn failed(&self) -> u32 {
            self.count_state(model::ActionState::Failed)
        }

        fn count_state(&self, state: model::ActionState) -> u32 {
            self.list
                .borrow()
                .iter()
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
    pub(crate) fn remove(&self, action: &model::Action) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _)) = list.shift_remove_full(action) {
            drop(list);
            self.items_changed(idx as u32, 1, 0);
        }
    }

    pub(crate) fn clear(&self) {
        let indexes = {
            let mut list = self.imp().list.borrow_mut();

            let indexes = list
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, action)| action.state() != model::ActionState::Ongoing)
                .map(|(i, _)| i)
                .collect::<Vec<_>>();

            indexes.iter().for_each(|i| {
                list.shift_remove_index(*i);
            });

            indexes
        };

        indexes.into_iter().for_each(|pos| {
            self.items_changed(pos as u32, 1, 0);
        });
    }

    fn notify_num_states(&self) {
        self.notify_ongoing();
        self.notify_finished();
        self.notify_cancelled();
        self.notify_failed();
    }
}

impl ActionList {
    pub(crate) fn build_image(
        &self,
        opts: engine::opts::ImageBuildOpts,
    ) -> model::ImageBuildAction {
        self.insert_action(model::ImageBuildAction::new(self, opts))
    }

    pub(crate) fn commit_container(
        &self,
        container: &model::Container,
        opts: engine::opts::ContainerCommitOpts,
    ) -> model::ContainerCommitAction {
        self.insert_action(model::ContainerCommitAction::new(self, container, opts))
    }

    pub(crate) fn copy_from_container(
        &self,
        container: &model::Container,
        container_path: &str,
        host_path: &str,
    ) -> model::ContainerCopyFromAction {
        self.insert_action(model::ContainerCopyFromAction::new(
            self,
            container,
            container_path,
            host_path,
        ))
    }

    pub(crate) fn copy_to_container(
        &self,
        container: &model::Container,
        directory: bool,
        host_path: &str,
        container_path: &str,
    ) -> model::ContainerCopyToAction {
        self.insert_action(model::ContainerCopyToAction::new(
            self,
            container,
            directory,
            host_path,
            container_path,
        ))
    }

    pub(crate) fn create_container(
        &self,
        opts: engine::opts::ContainerCreateOpts,
        run: bool,
    ) -> model::ContainerCreateAction {
        self.insert_action(model::ContainerCreateAction::new(self, opts, run, |_| {}))
    }

    pub(crate) fn create_pod(&self, opts: engine::opts::PodCreateOpts) -> model::PodCreateAction {
        self.insert_action(model::PodCreateAction::new(self, opts))
    }

    pub(crate) fn create_volume(
        &self,
        opts: engine::opts::VolumeCreateOpts,
    ) -> model::VolumeCreateAction {
        self.insert_action(model::VolumeCreateAction::new(self, opts))
    }

    pub(crate) fn pull_image(&self, opts: engine::opts::ImagePullOpts) -> model::ImagePullAction {
        self.insert_action(model::ImagePullAction::new(self, opts))
    }

    pub(crate) fn push_image(
        &self,
        repo_tag: &model::RepoTag,
        opts: engine::opts::ImagePushOpts,
    ) -> model::ImagePushAction {
        self.insert_action(model::ImagePushAction::new(self, repo_tag, opts))
    }

    pub(crate) fn prune_containers(
        &self,
        opts: engine::opts::ContainersPruneOpts,
    ) -> model::ContainersPruneAction {
        self.insert_action(model::ContainersPruneAction::new(self, opts))
    }

    pub(crate) fn prune_images(
        &self,
        opts: engine::opts::ImagesPruneOpts,
    ) -> model::ImagesPruneAction {
        self.insert_action(model::ImagesPruneAction::new(self, opts))
    }

    pub(crate) fn prune_pods(&self) -> model::PodsPruneAction {
        self.insert_action(model::PodsPruneAction::from(self))
    }

    pub(crate) fn prune_volumes(
        &self,
        opts: engine::opts::VolumesPruneOpts,
    ) -> model::VolumesPruneAction {
        self.insert_action(model::VolumesPruneAction::new(self, opts))
    }

    fn insert_action<A: IsA<model::Action>>(&self, action: A) -> A {
        let imp = self.imp();

        let position = {
            let mut list = imp.list.borrow_mut();
            list.insert(action.clone().upcast());
            list.len() - 1
        };

        action.connect_state_notify(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| obj.notify_num_states()
        ));

        self.items_changed(position as u32, 0, 1);

        action
    }
}
