use std::cell::Cell;

use gtk::glib;
use gtk::prelude::ObjectExt;
use gtk::prelude::ParamSpecBuilderExt;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct RepoTag {
        pub(super) repo_tag_list: glib::WeakRef<model::RepoTagList>,
        pub(super) full: OnceCell<String>,
        pub(super) to_be_deleted: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RepoTag {
        const NAME: &'static str = "RepoTag";
        type Type = super::RepoTag;
    }

    impl ObjectImpl for RepoTag {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::RepoTagList>("repo-tag-list")
                        .construct_only()
                        .build(),
                    glib::ParamSpecString::builder("full")
                        .construct_only()
                        .build(),
                    glib::ParamSpecString::builder("repo").read_only().build(),
                    glib::ParamSpecString::builder("tag").read_only().build(),
                    glib::ParamSpecBoolean::builder("to-be-deleted")
                        .explicit_notify()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "repo-tag-list" => self.repo_tag_list.set(value.get().unwrap()),
                "full" => self.full.set(value.get().unwrap()).unwrap(),
                "to-be-deleted" => self.obj().set_to_be_deleted(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "repo-tag-list" => obj.repo_tag_list().to_value(),
                "full" => obj.full().to_value(),
                "repo" => obj.repo().to_value(),
                "tag" => obj.tag().to_value(),
                "to-be-deleted" => obj.to_be_deleted().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct RepoTag(ObjectSubclass<imp::RepoTag>);
}

impl RepoTag {
    pub(crate) fn new(repo_tag_list: &model::RepoTagList, full: &str) -> Self {
        glib::Object::builder::<Self>()
            .property("repo-tag-list", repo_tag_list)
            .property("full", full)
            .build()
    }

    pub(crate) fn repo_tag_list(&self) -> Option<model::RepoTagList> {
        self.imp().repo_tag_list.upgrade()
    }

    pub(crate) fn full(&self) -> &str {
        self.imp().full.get().unwrap()
    }

    pub(crate) fn repo(&self) -> &str {
        self.full().split_once(':').unwrap().0
    }

    pub(crate) fn tag(&self) -> &str {
        self.full().split_once(':').unwrap().1
    }

    pub(crate) fn to_be_deleted(&self) -> bool {
        self.imp().to_be_deleted.get()
    }

    pub(crate) fn set_to_be_deleted(&self, value: bool) {
        if self.to_be_deleted() == value {
            return;
        }
        self.imp().to_be_deleted.set(value);
        self.notify("to-be-deleted");
    }
}
