use std::cell::RefCell;
use std::collections::HashSet;
use std::marker::PhantomData;

use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use indexmap::map::IndexMap;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::RepoTagList)]
    pub(crate) struct RepoTagList {
        pub(super) list: RefCell<IndexMap<String, model::RepoTag>>,

        #[property(get, set, construct_only, nullable)]
        pub(crate) image: glib::WeakRef<model::Image>,

        #[property(get = Self::len)]
        _len: PhantomData<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RepoTagList {
        const NAME: &'static str = "RepoTagList";
        type Type = super::RepoTagList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for RepoTagList {
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
            self.obj()
                .connect_items_changed(|obj, _, _, _| obj.notify_len());
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

    impl RepoTagList {
        pub(super) fn len(&self) -> u32 {
            self.list.borrow().len() as u32
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
    pub(crate) fn get(&self, index: usize) -> Option<model::RepoTag> {
        self.imp()
            .list
            .borrow()
            .get_index(index)
            .map(|(_, c)| c.clone())
    }

    pub(crate) fn contains(&self, lowercase_term: &str) -> bool {
        self.imp()
            .list
            .borrow()
            .keys()
            .any(|full| full.contains(lowercase_term))
    }

    pub(crate) fn add(&self, full: String) {
        let list = &self.imp().list;

        if list.borrow().contains_key(&full) {
            return;
        }

        let (index, _) = list
            .borrow_mut()
            .insert_full(full.clone(), model::RepoTag::new(self, full));

        self.items_changed(index as u32, 0, 1);
    }

    pub(crate) fn remove(&self, full: &str) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _, _)) = list.shift_remove_full(full) {
            drop(list);
            self.items_changed(idx as u32, 1, 0);
        }
    }

    pub(crate) fn update(&self, mut new_repo_tags: HashSet<String>) {
        let old_repo_tags = self.imp().list.borrow().keys().cloned().collect::<Vec<_>>();

        let intermediate_state_changed = new_repo_tags.is_empty() != old_repo_tags.is_empty();

        old_repo_tags.iter().for_each(|full| {
            if !new_repo_tags.remove(full) {
                self.remove(full);
            }
        });

        new_repo_tags.into_iter().for_each(|full| self.add(full));

        if intermediate_state_changed
            && let Some(image_list) = self.image().and_then(|image| image.image_list())
        {
            image_list.notify("intermediates");
        }
    }
}
