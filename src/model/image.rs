use std::borrow::Borrow;
use std::cell::Cell;
use std::cell::RefCell;
use std::ops::Deref;

use gtk::glib::clone;
use gtk::glib::subclass::Signal;
use gtk::glib::{self};
use gtk::prelude::ObjectExt;
use gtk::prelude::ParamSpecBuilderExt;
use gtk::prelude::ToValue;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::model;
use crate::podman;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct Image {
        pub(super) image_list: glib::WeakRef<model::ImageList>,

        pub(super) container_list: OnceCell<model::SimpleContainerList>,

        pub(super) containers: Cell<u64>,
        pub(super) created: OnceCell<i64>,
        pub(super) dangling: Cell<bool>,
        pub(super) id: OnceCell<String>,
        pub(super) repo_tags: RefCell<gtk::StringList>,
        pub(super) size: OnceCell<u64>,
        pub(super) shared_size: Cell<u64>,
        pub(super) virtual_size: Cell<u64>,

        pub(super) data: OnceCell<model::ImageData>,
        pub(super) can_inspect: Cell<bool>,

        pub(super) to_be_deleted: Cell<bool>,

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
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("deleted").build()]);
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::ImageList>("image-list")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecObject::builder::<model::SimpleContainerList>("container-list")
                        .read_only()
                        .build(),
                    glib::ParamSpecUInt64::builder("containers")
                        .read_only()
                        .build(),
                    glib::ParamSpecInt64::builder("created")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecBoolean::builder("dangling")
                        .read_only()
                        .build(),
                    glib::ParamSpecString::builder("id")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecObject::builder::<gtk::StringList>("repo-tags")
                        .read_only()
                        .build(),
                    glib::ParamSpecUInt64::builder("size")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecUInt64::builder("shared-size")
                        .read_only()
                        .build(),
                    glib::ParamSpecUInt64::builder("virtual-size")
                        .read_only()
                        .build(),
                    glib::ParamSpecObject::builder::<model::ImageData>("data")
                        .read_only()
                        .build(),
                    glib::ParamSpecBoolean::builder("to-be-deleted")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecBoolean::builder("selected").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "image-list" => self.image_list.set(value.get().unwrap()),
                "created" => self.created.set(value.get().unwrap()).unwrap(),
                "id" => self.id.set(value.get().unwrap()).unwrap(),
                "size" => self.size.set(value.get().unwrap()).unwrap(),
                "to-be-deleted" => self.obj().set_to_be_deleted(value.get().unwrap()),
                "selected" => self.selected.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "image-list" => obj.image_list().to_value(),
                "container-list" => obj.container_list().to_value(),
                "containers" => obj.containers().to_value(),
                "created" => obj.created().to_value(),
                "dangling" => obj.dangling().to_value(),
                "id" => obj.id().to_value(),
                "repo-tags" => obj.repo_tags().to_value(),
                "size" => obj.size().to_value(),
                "shared-size" => obj.shared_size().to_value(),
                "virtual-size" => obj.virtual_size().to_value(),
                "data" => obj.data().to_value(),
                "to-be-deleted" => obj.to_be_deleted().to_value(),
                "selected" => self.selected.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.can_inspect.set(true);
        }
    }
}

glib::wrapper! {
    pub(crate) struct Image(ObjectSubclass<imp::Image>) @implements model::Selectable;
}

impl Image {
    pub(crate) fn new(
        image_list: &model::ImageList,
        summary: &podman::models::LibpodImageSummary,
    ) -> Self {
        glib::Object::builder::<Self>()
            .property("image-list", image_list)
            .property("created", &summary.created.unwrap_or(0))
            .property("id", &summary.id)
            .property("size", &(summary.size.unwrap_or_default() as u64))
            .build()
            .update(summary)
            .to_owned()
    }

    pub(crate) fn update(&self, summary: &podman::models::LibpodImageSummary) -> &Self {
        self.set_containers(summary.containers.unwrap_or_default() as u64);
        self.set_dangling(summary.dangling.unwrap_or_default());
        self.set_repo_tags(gtk::StringList::new(
            &summary
                .repo_tags
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        ));
        self.set_shared_size(summary.shared_size.unwrap_or_default() as u64);
        self.set_virtual_size(summary.virtual_size.unwrap_or_default() as u64);

        self
    }

    pub(crate) fn image_list(&self) -> Option<model::ImageList> {
        self.imp().image_list.upgrade()
    }

    pub(crate) fn container_list(&self) -> &model::SimpleContainerList {
        self.imp().container_list.get_or_init(Default::default)
    }

    pub(crate) fn containers(&self) -> u64 {
        self.imp().containers.get()
    }

    fn set_containers(&self, value: u64) {
        if self.containers() == value {
            return;
        }
        self.imp().containers.set(value);
        self.notify("containers");
    }

    pub(crate) fn created(&self) -> i64 {
        *self.imp().created.get().unwrap()
    }

    pub(crate) fn dangling(&self) -> bool {
        self.imp().dangling.get()
    }

    fn set_dangling(&self, value: bool) {
        if self.dangling() == value {
            return;
        }
        self.imp().dangling.set(value);
        self.notify("dangling");
    }

    pub(crate) fn id(&self) -> &str {
        self.imp().id.get().unwrap()
    }

    pub(crate) fn repo_tags(&self) -> gtk::StringList {
        self.imp().repo_tags.borrow().to_owned()
    }

    fn set_repo_tags(&self, value: gtk::StringList) {
        if self.repo_tags() == value {
            return;
        }
        self.imp().repo_tags.replace(value);
        self.notify("repo-tags");
    }

    pub(crate) fn size(&self) -> u64 {
        *self.imp().size.get().unwrap()
    }

    pub(crate) fn shared_size(&self) -> u64 {
        self.imp().shared_size.get()
    }

    fn set_shared_size(&self, value: u64) {
        if self.shared_size() == value {
            return;
        }
        self.imp().shared_size.set(value);
        self.notify("shared-size");
    }

    pub(crate) fn virtual_size(&self) -> u64 {
        self.imp().virtual_size.get()
    }

    fn set_virtual_size(&self, value: u64) {
        if self.virtual_size() == value {
            return;
        }
        self.imp().virtual_size.set(value);
        self.notify("virtual-size");
    }

    pub(crate) fn data(&self) -> Option<&model::ImageData> {
        self.imp().data.get()
    }

    fn set_data(&self, value: model::ImageData) {
        if self.data().is_some() {
            return;
        }
        self.imp().data.set(value).unwrap();
        self.notify("data");
    }

    pub(crate) fn inspect<F>(&self, op: F)
    where
        F: FnOnce(podman::Error) + 'static,
    {
        let imp = self.imp();
        if !imp.can_inspect.get() {
            return;
        }

        imp.can_inspect.set(false);

        utils::do_async(
            {
                let image = self.api().unwrap();
                async move { image.inspect().await }
            },
            clone!(@weak self as obj => move |result| {
                match result {
                    Ok(data) => obj.set_data(model::ImageData::from(data)),
                    Err(e) => {
                        log::error!("Error on inspecting image '{}': {e}", obj.id());
                        op(e);
                    },
                }
                obj.imp().can_inspect.set(true);
            }),
        );
    }

    pub(crate) fn to_be_deleted(&self) -> bool {
        self.imp().to_be_deleted.get()
    }

    fn set_to_be_deleted(&self, value: bool) {
        if self.to_be_deleted() == value {
            return;
        }
        self.imp().to_be_deleted.set(value);
        self.notify("to-be-deleted");
    }
}

impl Image {
    pub(crate) fn add_container(&self, container: &model::Container) {
        self.container_list().add_container(container);
    }

    pub(crate) fn remove_container<Q: Borrow<str> + ?Sized>(&self, id: &Q) {
        self.container_list().remove_container(id);
    }

    pub(crate) fn delete<F>(&self, op: F)
    where
        F: FnOnce(&Self, podman::Result<()>) + 'static,
    {
        if let Some(image) = self.api() {
            self.set_to_be_deleted(true);

            utils::do_async(
                async move { image.remove().await },
                clone!(@weak self as obj => move |result| {
                    if let Err(ref e) = result {
                        obj.set_to_be_deleted(false);
                        log::error!("Error on removing image: {}", e);
                    }
                    op(&obj, result);
                }),
            );
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

    pub(crate) fn api(&self) -> Option<podman::api::Image> {
        self.image_list()
            .unwrap()
            .client()
            .map(|client| podman::api::Image::new(client.podman().deref().clone(), self.id()))
    }
}
