use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ArtifactAction)]
    pub(crate) struct ArtifactAction {
        #[property(get, set, construct_only, nullable)]
        pub(super) action_list: glib::WeakRef<model::ActionList2>,

        #[property(get, set, nullable)]
        pub(super) artifact: glib::WeakRef<glib::Object>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArtifactAction {
        const ABSTRACT: bool = true;
        const NAME: &'static str = "ArtifactAction";
        type Type = super::ArtifactAction;
        type ParentType = model::BaseAction;
    }

    impl ObjectImpl for ArtifactAction {
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
    pub(crate) struct ArtifactAction(ObjectSubclass<imp::ArtifactAction>)
        @extends model::BaseAction;
}

pub(crate) trait ArtifactActionExt: IsA<ArtifactAction> {
    fn set_artifact(&self, artifact: Option<&glib::Object>) {
        self.upcast_ref::<ArtifactAction>().set_artifact(artifact);
    }
}

impl<O: IsA<ArtifactAction>> ArtifactActionExt for O {}

unsafe impl<T: ObjectSubclass + ObjectImpl> IsSubclassable<T> for ArtifactAction {}
