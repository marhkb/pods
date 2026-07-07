use std::cell::OnceCell;

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
    #[properties(wrapper_type = super::ContainerCommitAction)]
    pub(crate) struct ContainerCommitAction {
        #[property(get, set, construct_only)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedContainerCommitOpts>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCommitAction {
        const NAME: &'static str = "ContainerCommitAction";
        type Type = super::ContainerCommitAction;
        type ParentType = model::Action;
    }

    impl ObjectImpl for ContainerCommitAction {
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
    pub(crate) struct ContainerCommitAction(ObjectSubclass<imp::ContainerCommitAction>)
        @extends model::Action;
}

impl ContainerCommitAction {
    pub(crate) fn new(
        action_list: &model::ActionList,
        container: &model::Container,
        opts: engine::opts::ContainerCommitOpts,
    ) -> Self {
        model::Action::builder::<Self>(action_list)
            .property("opts", model::BoxedContainerCommitOpts::from(opts))
            .property("container", container)
            .build()
            .exec()
    }

    fn exec(self) -> Self {
        let Some(api) = self.container().and_then(|container| container.api()) else {
            return self;
        };

        rt::Promise::new({
            let opts = (*self.opts()).clone();
            async move { api.commit(opts).await }
        })
        .defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |result| match result {
                Ok(_) => obj.set_state(model::ActionState::Finished),
                Err(e) => {
                    log::warn!("error pruning containers: {e}");
                    obj.set_failed(&e.to_string())
                }
            }
        ));

        self
    }
}
