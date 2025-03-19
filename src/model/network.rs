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

monad_boxed_type!(pub(crate) BoxedNetwork(podman::models::Network) impls Debug);

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Network)]
    pub(crate) struct Network {
        #[property(get, set, construct_only, nullable)]
        pub(super) network_list: glib::WeakRef<model::NetworkList>,
        #[property(get, set, construct_only)]
        pub(super) inner: OnceCell<BoxedNetwork>,
        #[property(get, set, construct_only)]
        pub(super) default: OnceCell<bool>,
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
    impl ObjectSubclass for Network {
        const NAME: &'static str = "Network";
        type Type = super::Network;
        type Interfaces = (model::Selectable,);
    }

    impl ObjectImpl for Network {
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
            obj.container_list().connect_items_changed(
                clone!(@weak obj => move |_, _, _, _| if let Some(network_list) = obj.network_list() {
                    network_list.notify_num_networks();
                }),
            );
        }
    }

    impl Network {
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
    pub(crate) struct Network(ObjectSubclass<imp::Network>) @implements model::Selectable;
}

impl Network {
    pub(crate) fn new(network_list: &model::NetworkList, inner: podman::models::Network) -> Self {
        glib::Object::builder()
            .property("network-list", network_list)
            .property("default", inner.name.as_ref().unwrap() == "podman")
            .property("inner", BoxedNetwork::from(inner))
            .build()
    }

    pub(crate) async fn delete(&self, force: bool) -> anyhow::Result<()> {
        if self.default() {
            return Err(anyhow::anyhow!("default network podman cannot be removed"))
        }
        let network = if let Some(network) = self.api() {
            network
        } else {
            return Ok(());
        };

        let imp = self.imp();

        imp.set_to_be_deleted(true);

        rt::Promise::new(async move {
            if force {
                network.remove().await
            } else {
                network.delete().await
            }
        })
        .exec()
        .await
        .inspect_err(|e| {
            imp.set_to_be_deleted(false);
            log::error!("Error on removing network: {}", e);
        })
        .map_err(anyhow::Error::from)
        .map(|_| ())
    }

    pub(crate) fn api(&self) -> Option<podman::api::Network> {
        self.network_list().unwrap().client().map(|client| {
            podman::api::Network::new(
                client.podman().deref().clone(),
                &self.inner().name.clone().unwrap(),
            )
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
