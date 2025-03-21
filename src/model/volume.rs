use std::cell::Cell;
use std::cell::OnceCell;
use std::ops::Deref;
use std::sync::OnceLock;

use gio::prelude::*;
use glib::Properties;
use glib::clone;
use glib::subclass::Signal;
use glib::subclass::prelude::*;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::monad_boxed_type;
use crate::podman;
use crate::rt;

monad_boxed_type!(pub(crate) BoxedVolume(podman::models::Volume) impls Debug);

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Volume)]
    pub(crate) struct Volume {
        #[property(get, set, construct_only, nullable)]
        pub(super) volume_list: glib::WeakRef<model::VolumeList>,
        #[property(get, set, construct_only)]
        pub(super) inner: OnceCell<BoxedVolume>,
        #[property(get, set)]
        pub(super) searching_containers: Cell<bool>,
        #[property(get, set)]
        pub(super) action_ongoing: Cell<bool>,
        #[property(get = Self::container_list)]
        pub(super) container_list: OnceCell<model::SimpleContainerList>,
        #[property(get)]
        pub(super) to_be_deleted: Cell<bool>,
        #[property(get, set)]
        pub(super) selected: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Volume {
        const NAME: &'static str = "Volume";
        type Type = super::Volume;
        type Interfaces = (model::Selectable,);
    }

    impl ObjectImpl for Volume {
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

        fn constructed(&self) {
            self.parent_constructed();
            let obj = &*self.obj();
            obj.container_list().connect_items_changed(clone!(
                #[weak]
                obj,
                move |_, _, _, _| if let Some(volume_list) = obj.volume_list() {
                    volume_list.notify_num_volumes();
                }
            ));
        }
    }

    impl Volume {
        pub(super) fn container_list(&self) -> model::SimpleContainerList {
            self.container_list.get_or_init(Default::default).to_owned()
        }

        pub(super) fn set_to_be_deleted(&self, value: bool) {
            let obj = &*self.obj();
            if obj.to_be_deleted() == value {
                return;
            }
            self.to_be_deleted.set(value);
            obj.notify("to-be-deleted");
        }
    }
}

glib::wrapper! {
    pub(crate) struct Volume(ObjectSubclass<imp::Volume>) @implements model::Selectable;
}

impl Volume {
    pub(crate) fn new(volume_list: &model::VolumeList, inner: podman::models::Volume) -> Self {
        glib::Object::builder()
            .property("volume-list", volume_list)
            .property("inner", BoxedVolume::from(inner))
            .build()
    }

    pub(crate) async fn delete(&self, force: bool) -> podman::Result<()> {
        let imp = self.imp();

        imp.set_to_be_deleted(true);

        rt::Promise::new({
            let volume = self.api().unwrap();
            async move {
                if force {
                    volume.remove().await
                } else {
                    volume.delete().await
                }
            }
        })
        .exec()
        .await
        .inspect_err(|e| {
            imp.set_to_be_deleted(false);
            log::error!("Error on removing volume: {}", e);
        })
    }

    pub(crate) fn api(&self) -> Option<podman::api::Volume> {
        self.volume_list().unwrap().client().map(|client| {
            podman::api::Volume::new(client.podman().deref().clone(), &self.inner().name)
        })
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
}
