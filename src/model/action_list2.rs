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
use crate::model::BaseActionExt;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ActionList2)]
    pub(crate) struct ActionList2 {
        pub(super) list: RefCell<IndexSet<model::BaseAction>>,

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
    impl ObjectSubclass for ActionList2 {
        const NAME: &'static str = "ActionList2";
        type Type = super::ActionList2;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for ActionList2 {
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

    impl ListModelImpl for ActionList2 {
        fn item_type(&self) -> glib::Type {
            model::BaseAction::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .map(model::BaseAction::upcast_ref)
                .cloned()
        }
    }

    impl ActionList2 {
        fn len(&self) -> u32 {
            self.n_items()
        }

        fn ongoing(&self) -> u32 {
            self.count_state(model::ActionState2::Ongoing)
        }

        fn finished(&self) -> u32 {
            self.count_state(model::ActionState2::Finished)
        }

        fn cancelled(&self) -> u32 {
            self.count_state(model::ActionState2::Cancelled)
        }

        fn failed(&self) -> u32 {
            self.count_state(model::ActionState2::Failed)
        }

        fn count_state(&self, state: model::ActionState2) -> u32 {
            self.list
                .borrow()
                .iter()
                .filter(|action| action.state() == state)
                .count() as u32
        }
    }
}

glib::wrapper! {
    pub(crate) struct ActionList2(ObjectSubclass<imp::ActionList2>)
        @implements gio::ListModel;
}

impl From<&model::Client> for ActionList2 {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl ActionList2 {
    pub(crate) fn remove(&self, action: &model::BaseAction) {
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
                .filter(|(_, action)| action.state() != model::ActionState2::Ongoing)
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

impl ActionList2 {
    pub(crate) fn create_container(
        &self,
        opts: engine::opts::ContainerCreateOpts,
        run: bool,
    ) -> model::ContainerCreateAction {
        self.insert_action(model::ContainerCreateAction::new(self, opts, run, |_| {}))
    }

    pub(crate) fn pull_image(&self, opts: engine::opts::ImagePullOpts) -> model::ImagePullAction {
        self.insert_action(model::ImagePullAction::new(self, opts))
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

    fn insert_action<A: IsA<model::BaseAction>>(&self, action: A) -> A {
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
