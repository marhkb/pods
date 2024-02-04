use std::borrow::Borrow;
use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::sync::OnceLock;

use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use glib::subclass::Signal;
use gtk::gio;
use gtk::glib;
use indexmap::IndexMap;
use indexmap::map::Entry;

use crate::model;
use crate::model::prelude::*;
use crate::podman;
use crate::rt;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::NetworkList)]
    pub(crate) struct NetworkList {
        pub(super) list: RefCell<IndexMap<String, model::Network>>,
        #[property(get, set)]
        pub(super) test: Cell<u32>,
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
    impl ObjectSubclass for NetworkList {
        const NAME: &'static str = "NetworkList";
        type Type = super::NetworkList;
        type Interfaces = (gio::ListModel, model::SelectableList);
    }

    impl ObjectImpl for NetworkList {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("network-added")
                        .param_types([model::Network::static_type()])
                        .build(),
                    Signal::builder("network-removed")
                        .param_types([model::Network::static_type()])
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
                        glib::ParamSpecUInt::builder("unused").read_only().build(),
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
                "unused" => self.obj().unused().to_value(),
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

            obj.connect_network_added(|list, _| list.notify_num_networks());
            obj.connect_network_removed(|list, _| list.notify_num_networks());
        }
    }

    impl ListModelImpl for NetworkList {
        fn item_type(&self) -> glib::Type {
            model::Network::static_type()
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

    impl NetworkList {
        pub(super) fn is_initialized(&self) -> bool {
            self.initialized.get().is_some()
        }

        pub(super) fn set_as_initialized(&self) {
            if self.is_initialized() {
                return;
            }
            self.initialized.set(()).unwrap();
            self.obj().notify("initialized");
        }

        pub(super) fn set_listing(&self, value: bool) {
            let obj = &*self.obj();
            if obj.listing() == value {
                return;
            }
            self.listing.set(value);
            obj.notify("listing");
        }
    }
}

glib::wrapper! {
    pub(crate) struct NetworkList(ObjectSubclass<imp::NetworkList>)
        @implements gio::ListModel, model::SelectableList;
}

impl From<&model::Client> for NetworkList {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl NetworkList {
    pub(crate) fn notify_num_networks(&self) {
        self.notify("unused");
        self.notify("used");
    }

    pub(crate) fn len(&self) -> u32 {
        self.n_items()
    }

    pub(crate) fn unused(&self) -> u32 {
        self.len() - self.used()
    }

    pub(crate) fn used(&self) -> u32 {
        self.imp()
            .list
            .borrow()
            .values()
            .filter(|network| network.container_list().n_items() > 0)
            .count() as u32
    }

    pub(crate) fn get_network<Q: Borrow<str> + ?Sized>(&self, id: &Q) -> Option<model::Network> {
        self.imp().list.borrow().get(id.borrow()).cloned()
    }

    pub(crate) fn remove_network(&self, id: &str) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _, network)) = list.shift_remove_full(id) {
            drop(list);

            self.items_changed(idx as u32, 1, 0);
            self.network_removed(&network);
            network.emit_deleted();
        }
    }

    fn network_added(&self, network: &model::Network) {
        self.emit_by_name::<()>("network-added", &[network]);
    }

    pub(crate) fn connect_network_added<F: Fn(&Self, &model::Network) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("network-added", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let network = values[1].get::<model::Network>().unwrap();
            f(&obj, &network);

            None
        })
    }

    fn network_removed(&self, network: &model::Network) {
        self.emit_by_name::<()>("network-removed", &[network]);
    }

    pub(crate) fn connect_network_removed<F: Fn(&Self, &model::Network) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("network-removed", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let network = values[1].get::<model::Network>().unwrap();
            f(&obj, &network);

            None
        })
    }

    pub(crate) fn refresh<F>(&self, id: Option<String>, err_op: F)
    where
        F: FnOnce(super::RefreshError) + Clone + 'static,
    {
        println!("ID: {:?}", id);

        self.imp().set_listing(true);

        rt::Promise::new({
            let podman = self.client().unwrap().podman();
            let id = id.clone();
            async move {
                podman
                    .networks()
                    .list(
                        &podman::opts::NetworkListOpts::builder()
                            .filter(
                                id.map(podman::Id::from)
                                    .map(podman::opts::NetworkListFilter::Id),
                            )
                            .build(),
                    )
                    .await
            }
        })
        .defer(clone!(@weak self as obj => move |result| {
            let imp = obj.imp();

            match result {
                Ok(networks) => {
                    if id.is_none() {
                        let to_remove = imp
                            .list
                            .borrow()
                            .keys()
                            .filter(|id| {
                                !networks
                                    .iter()
                                    .any(|network| network.id.as_ref() == Some(id))
                            })
                            .cloned()
                            .collect::<Vec<_>>();

                        to_remove.iter().for_each(|id| {
                            obj.remove_network(id);
                        });
                    }

                    networks.into_iter().for_each(|network| {
                        let index = obj.len();

                        let mut list = imp.list.borrow_mut();
                        if let Entry::Vacant(e) = list.entry(network.id.to_owned().unwrap()) {
                            let network = model::Network::new(&obj, network);
                            e.insert(network.clone());

                            drop(list);

                            obj.items_changed(index, 0, 1);
                            obj.network_added(&network);
                        }
                    });

                    println!("network {}", imp.list.borrow().len());
                }
                Err(e) => {
                    log::error!("Error on retrieving networks: {}", e);
                    err_op(super::RefreshError);
                }
            }

            imp.set_listing(false);
            imp.set_as_initialized();
        }));
    }

    pub(crate) fn handle_event<F>(&self, event: podman::models::Event, err_op: F)
    where
        F: FnOnce(super::RefreshError) + Clone + 'static,
    {
        let network_id = event.actor.id;

        match event.action.as_str() {
            "remove" => self.remove_network(&network_id),
            _ => self.refresh(self.get_network(&network_id).map(|_| network_id), err_op),
        }
    }
}
