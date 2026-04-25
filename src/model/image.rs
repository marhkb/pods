use std::cell::Cell;
use std::cell::OnceCell;
use std::sync::OnceLock;

use glib::Properties;
use glib::clone;
use glib::prelude::*;
use glib::subclass::Signal;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;
use crate::model;
use crate::rt;

mod imp {
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::Image)]
    pub(crate) struct Image {
        #[property(get, set, construct_only, nullable)]
        pub(super) image_list: glib::WeakRef<model::ImageList>,

        #[property(get = Self::container_list)]
        pub(super) container_list: OnceCell<model::SimpleContainerList>,

        #[property(get, set, construct_only)]
        pub(super) created: OnceCell<i64>,
        #[property(get, set, construct)]
        pub(super) dangling: Cell<bool>,
        #[property(get, set, construct_only)]
        pub(super) id: OnceCell<String>,
        #[property(get = Self::repo_tags)]
        pub(super) repo_tags: OnceCell<model::RepoTagList>,
        #[property(get, set, construct_only)]
        pub(super) size: OnceCell<u64>,

        #[property(get = Self::details, set, nullable)]
        pub(super) details: OnceCell<Option<model::ImageDetails>>,

        #[property(get)]
        pub(super) to_be_deleted: Cell<bool>,
        #[property(get, set)]
        pub(super) selected: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Image {
        const NAME: &'static str = "Image";
        type Type = super::Image;
        type Interfaces = (model::Selectable,);
    }

    impl ObjectImpl for Image {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("deleted").build()])
        }

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

    impl Image {
        pub(super) fn container_list(&self) -> model::SimpleContainerList {
            self.container_list.get_or_init(Default::default).to_owned()
        }

        pub(super) fn details(&self) -> Option<model::ImageDetails> {
            self.details.get().cloned().flatten()
        }

        pub(super) fn set_to_be_deleted(&self, value: bool) {
            let obj = &*self.obj();
            if obj.to_be_deleted() == value {
                return;
            }
            self.to_be_deleted.set(value);
            obj.notify_to_be_deleted();
        }

        pub(super) fn repo_tags(&self) -> model::RepoTagList {
            self.repo_tags
                .get_or_init(|| model::RepoTagList::from(&*self.obj()))
                .to_owned()
        }
    }
}

glib::wrapper! {
    pub(crate) struct Image(ObjectSubclass<imp::Image>) @implements model::Selectable;
}

impl Image {
    pub(crate) fn new(image_list: &model::ImageList, dto: engine::dto::Image) -> Self {
        match dto {
            engine::dto::Image::Summary(dto) => Self::new_from_summary(image_list, dto),
            engine::dto::Image::Inspection(dto) => Self::new_from_inspection(image_list, dto),
        }
    }

    pub(crate) fn new_from_summary(
        image_list: &model::ImageList,
        dto: engine::dto::ImageSummary,
    ) -> Self {
        Self::build(image_list, dto, |builder| builder)
    }

    pub(crate) fn new_from_inspection(
        image_list: &model::ImageList,
        dto: engine::dto::ImageInspection,
    ) -> Self {
        Self::build(image_list, dto.summary, |builder| {
            builder.property("details", Some(model::ImageDetails::from(dto.details)))
        })
    }

    fn build<F>(image_list: &model::ImageList, dto: engine::dto::ImageSummary, op: F) -> Self
    where
        F: FnOnce(glib::object::ObjectBuilder<Self>) -> glib::object::ObjectBuilder<Self>,
    {
        let obj = op(glib::Object::builder()
            .property("image-list", image_list)
            .property("created", dto.created)
            .property("dangling", dto.dangling)
            .property("id", dto.id)
            .property("size", dto.size))
        .build();

        obj.repo_tags().update(dto.repo_tags);

        obj
    }

    pub(crate) fn update(&self, dto: engine::dto::Image) {
        match dto {
            engine::dto::Image::Summary(dto) => self.update_from_summary(dto),
            engine::dto::Image::Inspection(dto) => self.update_from_inspection(dto),
        }
    }

    pub(crate) fn update_from_summary(&self, dto: engine::dto::ImageSummary) {
        self.set_dangling(dto.dangling);
        self.repo_tags().update(dto.repo_tags);
    }

    pub(crate) fn update_from_inspection(&self, dto: engine::dto::ImageInspection) {
        self.update_from_summary(dto.summary);

        match self.details() {
            Some(details) => details.update(dto.details),
            None => self.set_details(Some(model::ImageDetails::from(dto.details))),
        }
    }

    pub(crate) fn inspect_and_update<F>(&self, op: F)
    where
        F: Fn(anyhow::Error) + 'static,
    {
        let Some(api) = self.api() else { return };

        rt::Promise::new(async move { api.inspect().await }).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |dto| match dto {
                Ok(dto) => obj.update_from_inspection(dto),
                Err(e) => op(e),
            }
        ));
    }
}

impl Image {
    pub(crate) fn remove<F>(&self, force: bool, op: F)
    where
        F: FnOnce(&Self, anyhow::Result<()>) + 'static,
    {
        let Some(api) = self.api() else {
            return;
        };

        self.imp().set_to_be_deleted(true);

        rt::Promise::new(async move { api.remove(force).await }).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |result| {
                if let Err(ref e) = result {
                    obj.imp().set_to_be_deleted(false);
                    log::error!("Error on removing image: {}", e);
                }
                op(&obj, result);
            }
        ));
    }

    pub(crate) fn tagged(&self, tag: String) {
        let repo_tags = self.repo_tags();
        let repo_tags_len = repo_tags.len();
        repo_tags.add(tag);

        if repo_tags_len == 0
            && let Some(image_list) = self.image_list()
        {
            image_list.notify_num_images();
        }
    }

    pub(crate) fn untagged(&self, tag: &str) {
        let repo_tags = self.repo_tags();
        repo_tags.remove(tag);

        if repo_tags.len() == 0
            && let Some(image_list) = self.image_list()
        {
            image_list.notify_num_images();
        }
    }

    pub(super) fn emit_deleted(&self) {
        self.emit_by_name::<()>("deleted", &[]);
    }

    pub(crate) fn connect_deleted<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("deleted", true, move |values| {
            f(&values[0].get::<Self>().unwrap());

            None
        })
    }

    pub(crate) fn api(&self) -> Option<engine::api::Image> {
        self.image_list()
            .and_then(|client| client.api())
            .map(|api| api.get(self.id()))
    }
}
