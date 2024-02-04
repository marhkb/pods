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

use crate::engine;
use crate::model;
use crate::rt;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Network)]
    pub(crate) struct Network {
        #[property(get, set, construct_only, nullable)]
        pub(super) network_list: glib::WeakRef<model::NetworkList>,

        #[property(get = Self::container_list)]
        pub(super) container_list: OnceCell<model::SimpleContainerList>,

        #[property(get, set, construct_only)]
        pub(super) id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) default: OnceCell<bool>,
        #[property(get, set, construct_only)]
        pub(super) dns: OnceCell<bool>,
        #[property(get, set, nullable, construct_only)]
        pub(super) driver: OnceCell<Option<String>>,
        #[property(get, set, construct_only)]
        pub(super) internal: OnceCell<bool>,
        #[property(get, set, construct_only)]
        pub(super) ipv4: OnceCell<bool>,
        #[property(get, set, construct_only)]
        pub(super) ipv6: OnceCell<bool>,
        #[property(get, set, nullable, construct_only)]
        pub(super) name: OnceCell<Option<String>>,

        #[property(get, set)]
        pub(super) searching_containers: Cell<bool>,
        #[property(get, set)]
        pub(super) action_ongoing: Cell<bool>,
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
            obj.container_list().connect_items_changed(clone!(
                #[weak]
                obj,
                move |_, _, _, _| if let Some(network_list) = obj.network_list() {
                    network_list.notify_num_networks();
                }
            ));
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
    pub(crate) fn new(network_list: &model::NetworkList, dto: engine::dto::Network) -> Self {
        glib::Object::builder()
            .property("network-list", network_list)
            .property("id", dto.id)
            .property("default", dto.default)
            .property("dns", dto.dns)
            .property("driver", dto.driver)
            .property("internal", dto.internal)
            .property("ipv4", dto.ipv4)
            .property("ipv6", dto.ipv6)
            .property("name", dto.name)
            .build()
    }

    pub(crate) async fn remove(&self, force: bool) -> anyhow::Result<()> {
        if self.default() {
            return Err(anyhow::anyhow!("default network cannot be removed"));
        }
        let network = if let Some(network) = self.api() {
            network
        } else {
            return Ok(());
        };

        let imp = self.imp();

        imp.set_to_be_deleted(true);

        rt::Promise::new(async move { network.remove(force).await })
            .exec()
            .await
            .inspect_err(|e| {
                imp.set_to_be_deleted(false);
                log::error!("Error on removing network: {}", e);
            })
            .map_err(anyhow::Error::from)
            .map(|_| ())
    }

    pub(crate) fn api(&self) -> Option<engine::api::Network> {
        self.network_list()
            .and_then(|netwrork_list| netwrork_list.api())
            .map(|api| api.get(self.id()))
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
