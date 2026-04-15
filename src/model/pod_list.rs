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
use indexmap::map::IndexMap;

use crate::engine;
use crate::model;
use crate::model::SelectableListExt;
use crate::rt;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::PodList)]
    pub(crate) struct PodList {
        pub(super) list: RefCell<IndexMap<String, model::Pod>>,
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
    impl ObjectSubclass for PodList {
        const NAME: &'static str = "PodList";
        type Type = super::PodList;
        type Interfaces = (gio::ListModel, model::SelectableList);
    }

    impl ObjectImpl for PodList {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("pod-added")
                        .param_types([model::Pod::static_type()])
                        .build(),
                    Signal::builder("containers-in-pod-changed")
                        .param_types([model::Pod::static_type()])
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
                        glib::ParamSpecUInt::builder("degraded").read_only().build(),
                        glib::ParamSpecUInt::builder("not-running")
                            .read_only()
                            .build(),
                        glib::ParamSpecUInt::builder("paused").read_only().build(),
                        glib::ParamSpecUInt::builder("running").read_only().build(),
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
                "degraded" => self.obj().degraded().to_value(),
                "not-running" => self.obj().not_running().to_value(),
                "paused" => self.obj().paused().to_value(),
                "running" => self.obj().running().to_value(),
                "num-selected" => self.obj().num_selected().to_value(),
                _ => self.derived_property(id, pspec),
            }
        }
        fn constructed(&self) {
            self.parent_constructed();
            let obj = &*self.obj();
            model::SelectableList::bootstrap(obj.upcast_ref());
            obj.connect_items_changed(|self_, _, _, _| self_.notify("len"));

            obj.connect_pod_added(|list, pod| {
                pod.container_list().connect_items_changed(clone!(
                    #[weak]
                    list,
                    #[weak]
                    pod,
                    move |_, _, _, _| list.emit_by_name::<()>("containers-in-pod-changed", &[&pod])
                ));
            });
        }
    }

    impl ListModelImpl for PodList {
        fn item_type(&self) -> glib::Type {
            model::Pod::static_type()
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

    impl PodList {
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
    pub(crate) struct PodList(ObjectSubclass<imp::PodList>)
        @implements gio::ListModel, model::SelectableList;
}

impl From<&model::Client> for PodList {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl PodList {
    fn notify_num_pods(&self) {
        self.notify("degraded");
        self.notify("not-running");
        self.notify("paused");
        self.notify("running");
    }

    pub(crate) fn len(&self) -> u32 {
        self.n_items()
    }

    pub(crate) fn degraded(&self) -> u32 {
        self.num_pods_of_status(model::PodStatus::Degraded)
    }

    pub(crate) fn not_running(&self) -> u32 {
        self.len() - self.running() - self.paused() - self.degraded()
    }

    pub(crate) fn paused(&self) -> u32 {
        self.num_pods_of_status(model::PodStatus::Paused)
    }

    pub(crate) fn running(&self) -> u32 {
        self.num_pods_of_status(model::PodStatus::Running)
    }

    pub(crate) fn num_pods_of_status(&self, status: model::PodStatus) -> u32 {
        self.imp()
            .list
            .borrow()
            .values()
            .filter(|pod| pod.status() == status)
            .count() as u32
    }

    pub(crate) fn upsert_pod(&self, dto: engine::dto::Pod) {
        if let Some(pod) = self.get_pod(dto.id()) {
            pod.update(dto);
        } else {
            let pod = model::Pod::new(self, dto);

            let index = self.len();

            self.imp().list.borrow_mut().insert(pod.id(), pod.clone());

            self.items_changed(index, 0, 1);
            self.pod_added(&pod);
        }
    }

    pub(crate) fn get_pod(&self, id: &str) -> Option<model::Pod> {
        self.imp().list.borrow().get(id).cloned()
    }

    pub(crate) fn remove_pod(&self, id: &str) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _, pod)) = list.shift_remove_full(id) {
            drop(list);

            self.items_changed(idx as u32, 1, 0);
            self.notify_num_pods();
            pod.emit_deleted();
        }
    }

    pub(crate) fn api(&self) -> Option<engine::api::Pods> {
        self.client().map(|client| client.engine().pods())
    }

    pub(crate) fn connect_pod_added<F: Fn(&Self, &model::Pod) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_signal("pod-added", f)
    }

    pub(crate) fn connect_containers_in_pod_changed<F: Fn(&Self, &model::Pod) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_signal("containers-in-pod-changed", f)
    }

    fn connect_signal<F: Fn(&Self, &model::Pod) + 'static>(
        &self,
        signal: &str,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local(signal, true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let pod = values[1].get::<model::Pod>().unwrap();
            f(&obj, &pod);

            None
        })
    }
}

impl PodList {
    pub(crate) fn refresh<F>(&self, err_op: F)
    where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        let Some(api) = self.api() else { return };

        self.imp().set_listing(true);

        rt::Promise::new(async move { api.list().await }).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |dtos| {
                match dtos {
                    Ok(dtos) => {
                        let to_remove = obj
                            .imp()
                            .list
                            .borrow()
                            .keys()
                            .filter(|id| !dtos.iter().any(|dto| &dto.id == *id))
                            .cloned()
                            .collect::<Vec<_>>();
                        to_remove.iter().for_each(|id| {
                            obj.remove_pod(id);
                        });

                        dtos.into_iter()
                            .for_each(|dto| obj.upsert_pod(engine::dto::Pod::Summary(dto)));
                    }
                    Err(e) => {
                        log::error!("Error on retrieving pods: {}", e);
                        err_op(e);
                    }
                }
                let imp = obj.imp();
                imp.set_listing(false);
                imp.set_as_initialized();
            }
        ));
    }

    pub(crate) fn handle_event<F>(&self, event: engine::dto::Event, err_op: F)
    where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        match event {
            engine::Response::Docker(_) => unreachable!(),
            engine::Response::Podman(event) => match event.action.as_str() {
                "create" => self.upsert_pod_fetch(event.actor.id, err_op),
                "pause" => self.upsert_pod_status(event.actor.id, model::PodStatus::Paused, err_op),
                "remove" => self.remove_pod(&event.actor.id),
                "start" | "unpause" => {
                    self.upsert_pod_status(event.actor.id, model::PodStatus::Running, err_op)
                }
                "stop" => self.upsert_pod_status(event.actor.id, model::PodStatus::Stopped, err_op),
                _ => {}
            },
        }
    }

    fn pod_added(&self, pod: &model::Pod) {
        self.notify_num_pods();
        pod.connect_notify_local(
            Some("status"),
            clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, _| obj.notify_num_pods()
            ),
        );
        self.emit_by_name::<()>("pod-added", &[pod]);
    }

    fn upsert_pod_fetch<F>(&self, id: String, err_op: F)
    where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        let Some(api) = self.api().map(|api| api.get(id)) else {
            return;
        };

        rt::Promise::new(async move { api.inspect().await }).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |dto| match dto {
                Ok(dto) => obj.upsert_pod(engine::dto::Pod::Inspection(dto)),
                Err(e) => err_op(e),
            }
        ));
    }

    fn upsert_pod_status<F>(&self, id: String, status: model::PodStatus, err_op: F)
    where
        F: FnOnce(anyhow::Error) + Clone + 'static,
    {
        self.upsert_pod_with(id, |pod| pod.set_status(status), err_op);
    }

    fn upsert_pod_with<F, E>(&self, id: String, op: F, err_op: E)
    where
        F: FnOnce(&model::Pod),
        E: FnOnce(anyhow::Error) + Clone + 'static,
    {
        match self.get_pod(&id) {
            Some(pod) => op(&pod),
            None => self.upsert_pod_fetch(id.to_owned(), err_op),
        }
    }
}
