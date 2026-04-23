use std::cell::RefCell;

use adw::prelude::*;
use glib::Properties;
use glib::clone;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::model;
use crate::model::prelude::*;
use crate::rt;

mod imp {
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::PodsPruneAction)]
    pub(crate) struct PodsPruneAction {
        #[property(get, set, nullable)]
        pub(super) deleted_pods: RefCell<Option<gtk::StringList>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodsPruneAction {
        const NAME: &'static str = "PodsPruneAction";
        type Type = super::PodsPruneAction;
        type ParentType = model::BaseAction;
    }

    impl ObjectImpl for PodsPruneAction {
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
    pub(crate) struct PodsPruneAction(ObjectSubclass<imp::PodsPruneAction>)
        @extends model::BaseAction;
}

impl From<&model::ActionList2> for PodsPruneAction {
    fn from(value: &model::ActionList2) -> Self {
        model::BaseAction::builder::<Self>(value).build().exec()
    }
}

impl PodsPruneAction {
    fn exec(self) -> Self {
        let Some(api) = self
            .action_list()
            .and_then(|action_list| action_list.client())
            .map(|client| client.engine().pods())
        else {
            return self;
        };

        rt::Promise::new(async move { api.prune().await }).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |prune_report| match prune_report {
                Ok(prune_report) => {
                    obj.set_deleted_pods(Some(gtk::StringList::from_iter(prune_report.deleted)));
                    obj.set_state(model::ActionState2::Finished);
                }
                Err(e) => {
                    log::warn!("error pruning pods: {e}");
                    obj.set_failed(&e.to_string())
                }
            }
        ));

        self
    }
}
