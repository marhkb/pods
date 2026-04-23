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
    #[properties(wrapper_type = super::VolumeCreateAction)]
    pub(crate) struct VolumeCreateAction {
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedVolumeCreateOpts>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeCreateAction {
        const NAME: &'static str = "VolumeCreateAction";
        type Type = super::VolumeCreateAction;
        type ParentType = model::ArtifactAction;
    }

    impl ObjectImpl for VolumeCreateAction {
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
    pub(crate) struct VolumeCreateAction(ObjectSubclass<imp::VolumeCreateAction>)
        @extends model::BaseAction, model::ArtifactAction;
}

impl VolumeCreateAction {
    pub(crate) fn new(
        action_list: &model::ActionList2,
        opts: engine::opts::VolumeCreateOpts,
    ) -> Self {
        model::BaseAction::builder::<Self>(action_list)
            .property("opts", model::BoxedVolumeCreateOpts::from(opts))
            .build()
            .exec()
    }

    fn exec(self) -> Self {
        let Some(api) = self
            .action_list()
            .and_then(|action_list| action_list.client())
            .map(|client| client.engine().volumes())
        else {
            return self;
        };

        rt::Promise::new({
            let opts = (*self.opts()).clone();
            async move { api.create(opts).await }
        })
        .defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |name| match name {
                Ok(name) => {
                    obj.finish(
                        name,
                        clone!(
                            #[weak]
                            obj,
                            move |volume| {
                                obj.set_artifact(Some(volume.upcast_ref()));
                                obj.set_state(model::ActionState2::Finished);
                            }
                        ),
                    );
                }
                Err(e) => {
                    log::warn!("error creating volume: {e}");
                    obj.set_failed(&e.to_string())
                }
            }
        ));

        self
    }

    fn finish<F>(&self, name: String, op: F)
    where
        F: Fn(&model::Volume) + 'static,
    {
        let Some(volume_list) = self
            .action_list()
            .and_then(|action_list| action_list.client())
            .map(|client| client.volume_list())
        else {
            return;
        };

        match volume_list.get_volume(&name) {
            Some(volume) => op(&volume),
            None => {
                volume_list.connect_volume_added(move |_, volume| {
                    if volume.name() == name {
                        op(volume);
                    }
                });
            }
        }
    }
}
