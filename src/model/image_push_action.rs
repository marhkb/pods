use std::cell::OnceCell;

use adw::prelude::*;
use futures::StreamExt;
use futures::future;
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
    #[properties(wrapper_type = super::ImagePushAction)]
    pub(crate) struct ImagePushAction {
        #[property(get, set, construct_only)]
        pub(super) repo_tag: glib::WeakRef<model::RepoTag>,
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedImagePushOpts>,
        #[property(get)]
        pub(super) output: gtk::TextBuffer,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePushAction {
        const NAME: &'static str = "ImagePushAction";
        type Type = super::ImagePushAction;
        type ParentType = model::Action;
    }

    impl ObjectImpl for ImagePushAction {
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
    pub(crate) struct ImagePushAction(ObjectSubclass<imp::ImagePushAction>)
        @extends model::Action;
}

impl ImagePushAction {
    pub(crate) fn new(
        action_list: &model::ActionList,
        repo_tag: &model::RepoTag,
        opts: engine::opts::ImagePushOpts,
    ) -> Self {
        model::Action::builder::<Self>(action_list)
            .property("repo-tag", repo_tag)
            .property("opts", model::BoxedImagePushOpts::from(opts))
            .build()
            .exec()
    }

    fn exec(self) -> Self {
        let Some(api) = self
            .repo_tag()
            .and_then(|repo_tag| repo_tag.repo_tag_list())
            .and_then(|repo_tag_list| repo_tag_list.image())
            .and_then(|image| image.api())
        else {
            return self;
        };

        let opts = (*self.opts()).clone();
        let abort_registration = self.setup_abort_handle();

        rt::Pipe::new(api, move |api| {
            future::Abortable::new(api.push(opts), abort_registration).boxed()
        })
        .on_next(clone!(
            #[weak(rename_to = obj)]
            self,
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
                                log::warn!("error on pushing image: {error}");
                                obj.set_failed(&error);
                                glib::ControlFlow::Break
                            }
                            None => glib::ControlFlow::Continue,
                        },
                    },
                    Err(e) => {
                        log::warn!("error on pushing image: {e}");
                        obj.set_failed(&e.to_string());
                        glib::ControlFlow::Break
                    }
                }
            }
        ))
        .on_finish(clone!(
            #[weak(rename_to = obj)]
            self,
            move || if obj.state() != model::ActionState::Failed {
                obj.set_state(model::ActionState::Finished);
            }
        ));

        self
    }

    fn insert(&self, text: &str) {
        let output = self.output();
        let mut iter = output.end_iter();

        output.insert(&mut iter, text);
    }
}
