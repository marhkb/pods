use std::borrow::Borrow;
use std::cell::Cell;
use std::ops::Deref;

use gtk::glib::clone;
use gtk::glib::subclass::Signal;
use gtk::glib::WeakRef;
use gtk::glib::{self};
use gtk::prelude::ObjectExt;
use gtk::prelude::StaticType;
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
        pub(super) image_list: WeakRef<model::ImageList>,

        pub(super) container_list: OnceCell<model::SimpleContainerList>,

        pub(super) containers: Cell<u64>,
        pub(super) created: OnceCell<i64>,
        pub(super) dangling: Cell<bool>,
        pub(super) digest: OnceCell<String>,
        pub(super) history: OnceCell<utils::BoxedStringVec>,
        pub(super) id: OnceCell<String>,
        pub(super) parent_id: OnceCell<Option<String>>,
        pub(super) read_only: OnceCell<bool>,
        pub(super) repo_digests: OnceCell<utils::BoxedStringVec>,
        pub(super) repo_tags: OnceCell<utils::BoxedStringVec>,
        pub(super) size: OnceCell<u64>,
        pub(super) shared_size: OnceCell<u64>,
        pub(super) user: OnceCell<String>,
        pub(super) virtual_size: OnceCell<u64>,

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
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("inspection-failed", &[], <()>::static_type().into()).build(),
                    Signal::builder("deleted", &[], <()>::static_type().into()).build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "image-list",
                        "Image List",
                        "The parent image list",
                        model::ImageList::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecObject::new(
                        "container-list",
                        "Container List",
                        "The list of containers associated with this Image",
                        model::SimpleContainerList::static_type(),
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt64::new(
                        "containers",
                        "Containers",
                        "The number of containers of this Image",
                        u64::MIN,
                        u64::MAX,
                        u64::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT,
                    ),
                    glib::ParamSpecInt64::new(
                        "created",
                        "Created",
                        "The creation date time of this Image",
                        i64::MIN,
                        i64::MAX,
                        i64::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecBoolean::new(
                        "dangling",
                        "Dangling",
                        "Whether this Image is dangling",
                        bool::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT,
                    ),
                    glib::ParamSpecString::new(
                        "digest",
                        "Digest",
                        "The digest of this Image",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecBoxed::new(
                        "history",
                        "History",
                        "The history of this Image",
                        utils::BoxedStringVec::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "id",
                        "Id",
                        "The id of this Image",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "parent-id",
                        "Parent Id",
                        "The id of the parent Image",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecBoolean::new(
                        "read-only",
                        "Read Only",
                        "Whether this Image is read only",
                        bool::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecBoxed::new(
                        "repo-digests",
                        "Repo Digests",
                        "The repo digests of this Image",
                        utils::BoxedStringVec::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecBoxed::new(
                        "repo-tags",
                        "Repo Tags",
                        "The repo tags of this Image",
                        utils::BoxedStringVec::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecUInt64::new(
                        "size",
                        "Size",
                        "The size of this Image",
                        u64::MIN,
                        u64::MAX,
                        u64::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecUInt64::new(
                        "shared-size",
                        "Shared Size",
                        "The shared size of this Image",
                        u64::MIN,
                        u64::MAX,
                        u64::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "user",
                        "User",
                        "The user of this Image",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecUInt64::new(
                        "virtual-size",
                        "Virtual Size",
                        "The virtual size of this Image",
                        u64::MIN,
                        u64::MAX,
                        u64::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecObject::new(
                        "data",
                        "Data",
                        "the data of the image",
                        model::ImageData::static_type(),
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecBoolean::new(
                        "to-be-deleted",
                        "To Be Deleted",
                        "Whether this image is to be deleted",
                        bool::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecBoolean::new(
                        "selected",
                        "Selected",
                        "Whether this image is selected",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "image-list" => self.image_list.set(value.get().unwrap()),
                "containers" => self.containers.set(value.get().unwrap()),
                "created" => self.created.set(value.get().unwrap()).unwrap(),
                "dangling" => self.dangling.set(value.get().unwrap()),
                "digest" => self.digest.set(value.get().unwrap()).unwrap(),
                "history" => self.history.set(value.get().unwrap()).unwrap(),
                "id" => self.id.set(value.get().unwrap()).unwrap(),
                "parent-id" => self.parent_id.set(value.get().unwrap()).unwrap(),
                "read-only" => self.read_only.set(value.get().unwrap()).unwrap(),
                "repo-digests" => self.repo_digests.set(value.get().unwrap()).unwrap(),
                "repo-tags" => self.repo_tags.set(value.get().unwrap()).unwrap(),
                "size" => self.size.set(value.get().unwrap()).unwrap(),
                "shared-size" => self.shared_size.set(value.get().unwrap()).unwrap(),
                "user" => self.user.set(value.get().unwrap()).unwrap(),
                "virtual-size" => self.virtual_size.set(value.get().unwrap()).unwrap(),
                "to-be-deleted" => self.to_be_deleted.set(value.get().unwrap()),
                "selected" => self.selected.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image-list" => obj.image_list().to_value(),
                "container-list" => obj.container_list().to_value(),
                "containers" => obj.containers().to_value(),
                "created" => obj.created().to_value(),
                "dangling" => obj.dangling().to_value(),
                "digest" => obj.digest().to_value(),
                "history" => obj.history().to_value(),
                "id" => obj.id().to_value(),
                "parent-id" => obj.parent_id().to_value(),
                "read-only" => obj.read_only().to_value(),
                "repo-digests" => obj.repo_digests().to_value(),
                "repo-tags" => obj.repo_tags().to_value(),
                "size" => obj.size().to_value(),
                "shared-size" => obj.shared_size().to_value(),
                "user" => obj.user().to_value(),
                "virtual-size" => obj.virtual_size().to_value(),
                "data" => obj.data().to_value(),
                "to-be-deleted" => obj.to_be_deleted().to_value(),
                "selected" => self.selected.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
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
        summary: podman::models::LibpodImageSummary,
    ) -> Self {
        glib::Object::new(&[
            ("image-list", image_list),
            (
                "containers",
                &(summary.containers.unwrap_or_default() as u64),
            ),
            ("created", &summary.created.unwrap_or(0)),
            ("dangling", &summary.dangling.unwrap_or_default()),
            ("digest", summary.digest.as_ref().unwrap()),
            (
                "history",
                &utils::BoxedStringVec::from(summary.history.unwrap_or_default()),
            ),
            ("id", &summary.id),
            (
                "parent-id",
                &summary
                    .parent_id
                    .as_ref()
                    .map(|id| if id.is_empty() { None } else { Some(id) })
                    .unwrap_or_default(),
            ),
            ("read-only", &summary.read_only.unwrap_or_default()),
            (
                "repo-digests",
                &utils::BoxedStringVec::from(summary.repo_digests.unwrap_or_default()),
            ),
            (
                "repo-tags",
                &utils::BoxedStringVec::from(summary.repo_tags.unwrap_or_default()),
            ),
            ("size", &(summary.size.unwrap_or_default() as u64)),
            (
                "shared-size",
                &(summary.shared_size.unwrap_or_default() as u64),
            ),
            // FIXME: Find the right user in the response data.
            ("user", &glib::user_name().to_str().unwrap_or_default()),
            (
                "virtual-size",
                &(summary.virtual_size.unwrap_or_default() as u64),
            ),
        ])
        .expect("Failed to create Image")
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

    pub(crate) fn created(&self) -> i64 {
        *self.imp().created.get().unwrap()
    }

    pub(crate) fn dangling(&self) -> bool {
        self.imp().dangling.get()
    }

    pub(crate) fn digest(&self) -> &str {
        self.imp().digest.get().unwrap()
    }

    pub(crate) fn history(&self) -> &utils::BoxedStringVec {
        self.imp().history.get().unwrap()
    }

    pub(crate) fn id(&self) -> &str {
        self.imp().id.get().unwrap()
    }

    pub(crate) fn parent_id(&self) -> Option<&str> {
        self.imp().parent_id.get().unwrap().as_deref()
    }

    pub(crate) fn read_only(&self) -> bool {
        *self.imp().read_only.get().unwrap()
    }

    pub(crate) fn repo_digests(&self) -> &utils::BoxedStringVec {
        self.imp().repo_digests.get().unwrap()
    }

    pub(crate) fn repo_tags(&self) -> &utils::BoxedStringVec {
        self.imp().repo_tags.get().unwrap()
    }

    pub(crate) fn size(&self) -> u64 {
        *self.imp().size.get().unwrap()
    }

    pub(crate) fn shared_size(&self) -> u64 {
        *self.imp().shared_size.get().unwrap()
    }

    pub(crate) fn user(&self) -> &str {
        self.imp().user.get().unwrap()
    }

    pub(crate) fn virtual_size(&self) -> u64 {
        *self.imp().virtual_size.get().unwrap()
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

    pub(crate) fn inspect(&self) {
        let imp = self.imp();
        if !imp.can_inspect.get() {
            return;
        }

        imp.can_inspect.set(false);

        let podman = self
            .image_list()
            .unwrap()
            .client()
            .unwrap()
            .podman()
            .clone();
        let id = self.id().to_owned();

        utils::do_async(
            async move { podman.images().get(id).inspect().await },
            clone!(@weak self as obj => move |result| match result {
                Ok(data) => obj.set_data(model::ImageData::from(data)),
                Err(e) => {
                    log::error!("Error on inspecting image '{}': {e}", obj.id());
                    obj.emit_by_name::<()>("inspection-failed", &[]);
                    obj.imp().can_inspect.set(true);
                },
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
        if let Some(image) = self.api_image() {
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

    pub(crate) fn connect_inspection_failed<F: Fn(&Self) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("inspection-failed", true, move |values| {
            f(&values[0].get::<Self>().unwrap());

            None
        })
    }

    pub(crate) fn connect_deleted<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("deleted", true, move |values| {
            f(&values[0].get::<Self>().unwrap());

            None
        })
    }

    pub(crate) fn api_image(&self) -> Option<podman::api::Image> {
        self.image_list()
            .unwrap()
            .client()
            .map(|client| podman::api::Image::new(client.podman().deref().clone(), self.id()))
    }
}
