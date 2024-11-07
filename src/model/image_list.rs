use std::borrow::Borrow;
use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::sync::OnceLock;

use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::clone;
use glib::subclass::Signal;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use indexmap::map::Entry;
use indexmap::IndexMap;

use crate::model;
use crate::model::SelectableListExt;
use crate::podman;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ImageList)]
    pub(crate) struct ImageList {
        pub(super) list: RefCell<IndexMap<String, model::Image>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get)]
        pub(super) listing: Cell<bool>,
        #[property(get = Self::is_initialized, type = bool)]
        pub(super) initialized: OnceCell<()>,
        #[property(get, set)]
        pub(super) selection_mode: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageList {
        const NAME: &'static str = "ImageList";
        type Type = super::ImageList;
        type Interfaces = (gio::ListModel, model::SelectableList);
    }

    impl ObjectImpl for ImageList {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("image-added")
                        .param_types([model::Image::static_type()])
                        .build(),
                    Signal::builder("image-removed")
                        .param_types([model::Image::static_type()])
                        .build(),
                    Signal::builder("containers-of-image-changed")
                        .param_types([model::Image::static_type()])
                        .build(),
                    Signal::builder("tags-of-image-changed")
                        .param_types([model::Image::static_type()])
                        .build(),
                ]
            })
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                Self::derived_properties()
                    .iter()
                    .cloned()
                    .chain(vec![
                        glib::ParamSpecUInt::builder("len").read_only().build(),
                        glib::ParamSpecUInt::builder("intermediates")
                            .read_only()
                            .build(),
                        glib::ParamSpecUInt::builder("used").read_only().build(),
                        glib::ParamSpecUInt::builder("num-selected")
                            .read_only()
                            .build(),
                    ])
                    .collect::<Vec<_>>()
            })
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "len" => self.obj().len().to_value(),
                "intermediates" => self.obj().intermediates().to_value(),
                "used" => self.obj().used().to_value(),
                "num-selected" => self.obj().num_selected().to_value(),
                _ => self.derived_property(id, pspec),
            }
        }
        fn constructed(&self) {
            self.parent_constructed();
            let obj = &*self.obj();

            model::SelectableList::bootstrap(obj.upcast_ref());

            obj.connect_items_changed(|self_, _, _, _| self_.notify("len"));
        }
    }

    impl ListModelImpl for ImageList {
        fn item_type(&self) -> glib::Type {
            model::Image::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .map(|(_, obj)| obj.upcast_ref())
                .cloned()
        }
    }

    impl ImageList {
        pub(super) fn is_initialized(&self) -> bool {
            self.initialized.get().is_some()
        }

        pub(super) fn set_as_initialized(&self) {
            if self.is_initialized() {
                return;
            }
            self.initialized.set(()).unwrap();
            self.obj().notify_initialized();
        }

        pub(super) fn set_listing(&self, value: bool) {
            let obj = &*self.obj();
            if obj.listing() == value {
                return;
            }
            self.listing.set(value);
            obj.notify_listing();
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageList(ObjectSubclass<imp::ImageList>)
        @implements gio::ListModel, model::SelectableList;
}

impl From<&model::Client> for ImageList {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl ImageList {
    pub(crate) fn notify_num_images(&self) {
        self.notify("intermediates");
        self.notify("used");
    }

    pub(crate) fn len(&self) -> u32 {
        self.n_items()
    }

    pub(crate) fn intermediates(&self) -> u32 {
        self.imp()
            .list
            .borrow()
            .values()
            .filter(|image| image.repo_tags().n_items() == 0)
            .count() as u32
    }

    pub(crate) fn used(&self) -> u32 {
        self.len() - self.intermediates()
    }

    pub(crate) fn total_size(&self) -> u64 {
        self.imp()
            .list
            .borrow()
            .values()
            .map(model::Image::size)
            .sum()
    }

    pub(crate) fn unused_size(&self) -> u64 {
        self.imp()
            .list
            .borrow()
            .values()
            .filter(|image| image.repo_tags().n_items() == 0)
            .map(model::Image::size)
            .sum()
    }

    pub(crate) fn get_image<Q: Borrow<str> + ?Sized>(&self, id: &Q) -> Option<model::Image> {
        self.imp().list.borrow().get(id.borrow()).cloned()
    }

    pub(crate) fn remove_image(&self, id: &str) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _, image)) = list.shift_remove_full(id) {
            drop(list);

            self.items_changed(idx as u32, 1, 0);
            self.emit_by_name::<()>("image-removed", &[&image]);
            self.notify_num_images();
            image.emit_deleted();
        }
    }

    pub(crate) fn refresh<F>(&self, err_op: F)
    where
        F: FnOnce(super::RefreshError) + Clone + 'static,
    {
        self.imp().set_listing(true);
        utils::do_async(
            {
                let podman = self.client().unwrap().podman();
                async move {
                    podman
                        .images()
                        .list(&podman::opts::ImageListOpts::builder().all(true).build())
                        .await
                }
            },
            clone!(
                #[weak(rename_to = obj)]
                self,
                move |result| {
                    match result {
                        Ok(summaries) => {
                            let to_remove = obj
                                .imp()
                                .list
                                .borrow()
                                .keys()
                                .filter(|id| {
                                    !summaries
                                        .iter()
                                        .any(|summary| summary.id.as_ref() == Some(id))
                                })
                                .cloned()
                                .collect::<Vec<_>>();
                            to_remove.iter().for_each(|id| {
                                obj.remove_image(id);
                            });

                            summaries.iter().for_each(|summary| {
                                let index = obj.len();

                                let mut list = obj.imp().list.borrow_mut();

                                match list.entry(summary.id.as_ref().unwrap().to_owned()) {
                                    Entry::Vacant(e) => {
                                        let image = model::Image::new(&obj, summary);
                                        e.insert(image.clone());

                                        drop(list);

                                        obj.items_changed(index, 0, 1);
                                        obj.image_added(&image);
                                    }
                                    Entry::Occupied(e) => {
                                        let image = e.get().to_owned();
                                        drop(list);
                                        image.update(summary);
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            log::error!("Error on retrieving images: {}", e);
                            err_op(super::RefreshError);
                        }
                    }
                    let imp = obj.imp();
                    imp.set_listing(false);
                    imp.set_as_initialized();
                }
            ),
        );
    }

    fn tag(&self, id: &str, tag: &str) {
        if let Some(image) = self.imp().list.borrow().get(id) {
            let repo_tags = image.repo_tags();
            let repo_tags_len = repo_tags.len();
            repo_tags.add(model::RepoTag::new(&repo_tags, tag));

            if repo_tags_len == 0 {
                self.notify_num_images();
            }
        }
    }

    fn untag(&self, id: &str, tag: &str) {
        if let Some(image) = self.imp().list.borrow().get(id) {
            let repo_tags = image.repo_tags();
            repo_tags.remove(tag);

            if repo_tags.len() == 0 {
                self.notify_num_images();
            }
        }
    }

    pub(crate) fn handle_event<F>(&self, event: podman::models::Event, err_op: F)
    where
        F: FnOnce(super::RefreshError) + Clone + 'static,
    {
        match event.action.as_str() {
            "tag" => self.tag(
                &event.actor.id,
                &format!("localhost/{}", event.actor.attributes.get("name").unwrap()),
            ),
            "untag" => self.untag(&event.actor.id, event.actor.attributes.get("name").unwrap()),
            "remove" => self.remove_image(&event.actor.id),
            "build" | "pull" => self.refresh(err_op),
            other => log::warn!("Unknown action: {other}"),
        }
    }

    fn image_added(&self, image: &model::Image) {
        self.notify_num_images();
        image.connect_notify_local(
            Some("repo-tags"),
            clone!(
                #[weak(rename_to = obj)]
                self,
                move |image, _| {
                    obj.notify_num_images();
                    obj.emit_by_name::<()>("containers-of-image-changed", &[&image]);
                    obj.emit_by_name::<()>("tags-of-image-changed", &[&image]);
                }
            ),
        );
        self.emit_by_name::<()>("image-added", &[image]);
    }

    pub(crate) fn connect_image_added<F: Fn(&Self, &model::Image) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_signal("image-added", f)
    }

    pub(crate) fn connect_containers_of_image_changed<F: Fn(&Self, &model::Image) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_signal("containers-of-image-changed", f)
    }

    pub(crate) fn connect_tags_of_image_changed<F: Fn(&Self, &model::Image) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_signal("tags-of-image-changed", f)
    }

    fn connect_signal<F: Fn(&Self, &model::Image) + 'static>(
        &self,
        signal: &str,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local(signal, true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let image = values[1].get::<model::Image>().unwrap();
            f(&obj, &image);

            None
        })
    }
}
