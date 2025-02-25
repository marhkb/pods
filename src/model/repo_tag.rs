use std::cell::Cell;
use std::cell::OnceCell;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::RepoTag)]
    pub(crate) struct RepoTag {
        #[property(get, set, construct_only, nullable)]
        pub(super) repo_tag_list: glib::WeakRef<model::RepoTagList>,
        #[property(get, set, construct_only)]
        pub(super) full: OnceCell<String>,
        #[property(get, set)]
        pub(super) to_be_deleted: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RepoTag {
        const NAME: &'static str = "RepoTag";
        type Type = super::RepoTag;
    }

    impl ObjectImpl for RepoTag {
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
    pub(crate) struct RepoTag(ObjectSubclass<imp::RepoTag>);
}

impl RepoTag {
    pub(crate) fn new(repo_tag_list: &model::RepoTagList, full: &str) -> Self {
        glib::Object::builder()
            .property("repo-tag-list", repo_tag_list)
            .property("full", full)
            .build()
    }

    pub(crate) fn host(&self) -> String {
        self.full().split_once('/').unwrap().0.to_owned()
    }

    pub(crate) fn namespace(&self) -> String {
        self.full().split_once('/').unwrap().1.to_owned()
    }

    pub(crate) fn repo(&self) -> String {
        self.full().split_once(':').unwrap().0.to_owned()
    }

    pub(crate) fn tag(&self) -> String {
        self.full().split_once(':').unwrap().1.to_owned()
    }
}
