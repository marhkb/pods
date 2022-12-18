use std::cell::RefCell;
use std::collections::HashSet;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use indexmap::map::IndexMap;
use once_cell::sync::Lazy;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct RepoTagList {
        pub(crate) image: glib::WeakRef<model::Image>,
        pub(super) list: RefCell<IndexMap<String, model::RepoTag>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RepoTagList {
        const NAME: &'static str = "RepoTagList";
        type Type = super::RepoTagList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for RepoTagList {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Image>("image")
                        .construct_only()
                        .build(),
                    glib::ParamSpecUInt::builder("len").read_only().build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "image" => self.image.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "image" => obj.image().to_value(),
                "len" => obj.len().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj()
                .connect_items_changed(|obj, _, _, _| obj.notify("len"));
        }
    }

    impl ListModelImpl for RepoTagList {
        fn item_type(&self) -> glib::Type {
            model::RepoTag::static_type()
        }

        fn n_items(&self) -> u32 {
            self.obj().len()
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.obj().get(position as usize).map(|obj| obj.upcast())
        }
    }
}

glib::wrapper! {
    pub(crate) struct RepoTagList(ObjectSubclass<imp::RepoTagList>)
        @implements gio::ListModel, model::AbstractContainerList;
}

impl From<&model::Image> for RepoTagList {
    fn from(image: &model::Image) -> Self {
        glib::Object::builder().property("image", image).build()
    }
}

impl RepoTagList {
    pub(crate) fn image(&self) -> Option<model::Image> {
        self.imp().image.upgrade()
    }

    pub(crate) fn get(&self, index: usize) -> Option<model::RepoTag> {
        self.imp()
            .list
            .borrow()
            .get_index(index)
            .map(|(_, c)| c.clone())
    }

    pub(crate) fn contains(&self, uppercase_term: &str) -> bool {
        self.imp()
            .list
            .borrow()
            .keys()
            .any(|full| full.to_uppercase().contains(uppercase_term))
    }

    pub(crate) fn add(&self, repo_tag: model::RepoTag) {
        let (index, _) = self
            .imp()
            .list
            .borrow_mut()
            .insert_full(repo_tag.full().to_owned(), repo_tag);

        self.items_changed(index as u32, 0, 1);
    }

    pub(crate) fn remove(&self, full: &str) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _, _)) = list.shift_remove_full(full) {
            drop(list);
            self.items_changed(idx as u32, 1, 0);
        }
    }

    pub(crate) fn update(&self, mut new_repo_tags: HashSet<&String>) -> bool {
        let old_repo_tags = self.imp().list.borrow().keys().cloned().collect::<Vec<_>>();

        let intermediate_state_changed = new_repo_tags.is_empty() != old_repo_tags.is_empty();

        old_repo_tags.iter().for_each(|full| {
            if !new_repo_tags.remove(full) {
                self.remove(full);
            }
        });

        new_repo_tags
            .into_iter()
            .map(String::as_str)
            .map(|full| model::RepoTag::new(self, full))
            .for_each(|repo_tag| self.add(repo_tag));

        intermediate_state_changed
    }

    pub(crate) fn len(&self) -> u32 {
        self.imp().list.borrow().len() as u32
    }
}
