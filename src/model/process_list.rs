use std::cell::RefCell;
use std::sync::OnceLock;

use futures::StreamExt;
use futures::TryStreamExt;
use futures::stream;
use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use glib::subclass::Signal;
use gtk::gio;
use gtk::glib;
use indexmap::map::IndexMap;

use crate::model;
use crate::podman;

mod imp {
    use super::*;
    use crate::rt;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ProcessList)]
    pub(crate) struct ProcessList {
        pub(super) list: RefCell<IndexMap<String, model::Process>>,
        #[property(get, set, construct_only, nullable)]
        /// A `Container` or a `Pod`
        pub(super) top_source: glib::WeakRef<glib::Object>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProcessList {
        const NAME: &'static str = "ProcessList";
        type Type = super::ProcessList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for ProcessList {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("updated").build()])
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

            let processes_source = if let Some(processes_source) =
                obj.top_source().as_ref().and_then(|obj| {
                    if let Some(container) = obj.downcast_ref::<model::Container>() {
                        container.api().map(|c| Box::new(c) as Box<dyn TopSource>)
                    } else if let Some(pod) = obj.downcast_ref::<model::Pod>() {
                        pod.api().map(|p| Box::new(p) as Box<dyn TopSource>)
                    } else {
                        unreachable!("unknown type for top source: {obj:?}")
                    }
                }) {
                processes_source
            } else {
                return;
            };

            rt::Pipe::new(processes_source, |top_source| top_source.stream()).on_next(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move |result: podman::Result<ProcessFields>| {
                    match result {
                        Ok(process_fields) => {
                            let to_remove = obj
                                .imp()
                                .list
                                .borrow()
                                .keys()
                                .filter(|pid| {
                                    !process_fields
                                        .iter()
                                        .any(|process_field| &process_field[1] == *pid)
                                })
                                .cloned()
                                .collect::<Vec<_>>();
                            to_remove.iter().for_each(|pid| {
                                obj.remove(pid);
                            });

                            process_fields.into_iter().for_each(|process_field| {
                                use indexmap::map::Entry;

                                let mut list = obj.imp().list.borrow_mut();
                                let index = list.len() as u32;

                                match list.entry(process_field[1].clone()) {
                                    Entry::Vacant(e) => {
                                        let process =
                                            model::Process::new(&obj, process_field.as_slice());
                                        e.insert(process.clone());

                                        drop(list);

                                        obj.items_changed(index, 0, 1);
                                    }
                                    Entry::Occupied(e) => {
                                        let process = e.get().clone();
                                        drop(list);
                                        process.update(process_field.as_slice());
                                    }
                                }
                            });

                            obj.emit_by_name::<()>("updated", &[]);
                        }
                        Err(e) => log::warn!("Failed to read top stream element: {e}"),
                    }

                    glib::ControlFlow::Continue
                }
            ));
        }
    }

    impl ListModelImpl for ProcessList {
        fn item_type(&self) -> glib::Type {
            model::Process::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .map(|(_, item)| item.upcast_ref())
                .cloned()
        }
    }
}

glib::wrapper! {
    pub(crate) struct ProcessList(ObjectSubclass<imp::ProcessList>)
        @implements gio::ListModel, model::AbstractContainerList;
}

impl From<&model::Container> for ProcessList {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder()
            .property("top-source", container)
            .build()
    }
}

impl From<&model::Pod> for ProcessList {
    fn from(pod: &model::Pod) -> Self {
        glib::Object::builder().property("top-source", pod).build()
    }
}

impl ProcessList {
    fn remove(&self, pid: &str) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _, _)) = list.shift_remove_full(pid) {
            drop(list);
            self.items_changed(idx as u32, 1, 0);
        }
    }

    pub(crate) fn connect_updated<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("updated", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);

            None
        })
    }
}

trait TopSource: Send {
    fn stream(&self) -> stream::BoxStream<podman::Result<ProcessFields>>;
}

impl TopSource for podman::api::Container {
    fn stream(&self) -> stream::BoxStream<podman::Result<ProcessFields>> {
        self.top_stream(&podman::opts::ContainerTopOpts::builder().delay(1).build())
            .map_ok(|top| top.processes)
            .boxed()
    }
}

impl TopSource for podman::api::Pod {
    fn stream(&self) -> stream::BoxStream<podman::Result<ProcessFields>> {
        self.top_stream(&podman::opts::PodTopOpts::builder().delay(1).build())
            .map_ok(|top| top.processes)
            .boxed()
    }
}

type ProcessFields = Vec<Vec<String>>;
