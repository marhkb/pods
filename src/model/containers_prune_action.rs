use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;

use adw::prelude::*;
use glib::Properties;
use glib::clone;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;
use crate::model;
use crate::model::prelude::*;
use crate::rt;

mod imp {
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ContainersPruneAction)]
    pub(crate) struct ContainersPruneAction {
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedContainersPruneOpts>,
        #[property(get, set, nullable)]
        pub(super) deleted_containers: RefCell<Option<gtk::StringList>>,
        #[property(get, set)]
        pub(super) space_reclaimed: Cell<u64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainersPruneAction {
        const NAME: &'static str = "ContainersPruneAction";
        type Type = super::ContainersPruneAction;
        type ParentType = model::BaseAction;
    }

    impl ObjectImpl for ContainersPruneAction {
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
    pub(crate) struct ContainersPruneAction(ObjectSubclass<imp::ContainersPruneAction>)
        @extends model::BaseAction;
}

impl ContainersPruneAction {
    pub(crate) fn new(
        action_list: &model::ActionList2,
        opts: engine::opts::ContainersPruneOpts,
    ) -> Self {
        model::BaseAction::builder::<Self>(action_list)
            .property("opts", model::BoxedContainersPruneOpts::from(opts))
            .build()
            .exec()
    }

    fn exec(self) -> Self {
        let Some(api) = self
            .action_list()
            .and_then(|action_list| action_list.client())
            .map(|client| client.engine().containers())
        else {
            return self;
        };

        rt::Promise::new({
            let opts = (*self.opts()).clone();
            async move { api.prune(opts).await }
        })
        .defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |prune_report| match prune_report {
                Ok(prune_report) => {
                    obj.set_deleted_containers(Some(gtk::StringList::from_iter(
                        prune_report.deleted,
                    )));
                    obj.set_space_reclaimed(prune_report.space_reclaimed);
                    obj.set_state(model::ActionState2::Finished);
                }
                Err(e) => {
                    log::warn!("error pruning containers: {e}");
                    obj.set_failed(&e.to_string())
                }
            }
        ));

        self
    }
}
