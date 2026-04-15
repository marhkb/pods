use std::cell::RefCell;
use std::sync::OnceLock;

use futures::stream;
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

mod imp {
    use super::*;
    use crate::rt;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ProcessList)]
    pub(crate) struct ProcessList {
        pub(super) list: RefCell<IndexMap<i32, model::Process>>,
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
                move |top| {
                    match top {
                        Ok(top) => {
                            let imp = obj.imp();

                            let to_remove = imp
                                .list
                                .borrow()
                                .keys()
                                .cloned()
                                .filter(|pid| {
                                    !top.processes().iter().any(|process| process.pid == *pid)
                                })
                                .collect::<Vec<_>>();
                            to_remove.into_iter().for_each(|pid| {
                                obj.remove(pid);
                            });

                            top.into_processes().into_iter().for_each(|process| {
                                use indexmap::map::Entry;

                                let items_changed = match imp.list.borrow_mut().entry(process.pid) {
                                    Entry::Vacant(e) => {
                                        e.insert(model::Process::new(&obj, process));
                                        true
                                    }
                                    Entry::Occupied(e) => {
                                        e.get().update(process);
                                        false
                                    }
                                };

                                if items_changed {
                                    obj.items_changed(obj.n_items() - 1, 0, 1);
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
    fn remove(&self, pid: i32) {
        let mut list = self.imp().list.borrow_mut();
        if let Some((idx, _, _)) = list.shift_remove_full(&pid) {
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
    fn stream(&'_ self) -> stream::BoxStream<'_, anyhow::Result<engine::dto::Top>>;
}

impl TopSource for engine::api::Container {
    fn stream(&'_ self) -> stream::BoxStream<'_, anyhow::Result<engine::dto::Top>> {
        self.top_stream(1)
    }
}

impl TopSource for engine::api::Pod {
    fn stream(&'_ self) -> stream::BoxStream<'_, anyhow::Result<engine::dto::Top>> {
        self.top_stream(1)
    }
}
