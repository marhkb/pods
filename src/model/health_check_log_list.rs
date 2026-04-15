use std::cell::RefCell;
use std::collections::VecDeque;

use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::Properties;
use gtk::gio;
use gtk::glib;

use crate::engine;
use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::HealthCheckLogList)]
    pub(crate) struct HealthCheckLogList {
        pub(super) list: RefCell<VecDeque<model::HealthCheckLog>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HealthCheckLogList {
        const NAME: &'static str = "HealthCheckLogList";
        type Type = super::HealthCheckLogList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for HealthCheckLogList {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }

    impl ListModelImpl for HealthCheckLogList {
        fn item_type(&self) -> glib::Type {
            model::HealthCheckLog::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get(position as usize)
                .map(|obj| obj.upcast_ref())
                .cloned()
        }
    }
}

glib::wrapper! {
    pub(crate) struct HealthCheckLogList(ObjectSubclass<imp::HealthCheckLogList>)
        @implements gio::ListModel, model::SelectableList;
}

impl From<Vec<engine::dto::HealthCheckLog>> for HealthCheckLogList {
    fn from(value: Vec<engine::dto::HealthCheckLog>) -> Self {
        let obj: Self = glib::Object::new();
        obj.sync(value);
        obj
    }
}

impl HealthCheckLogList {
    pub(crate) fn sync(&self, log: Vec<engine::dto::HealthCheckLog>) {
        let mut list = self.imp().list.borrow_mut();

        let len_old = list.len();

        let first = log.first().and_then(|log| log.start.as_deref());
        while list.front().is_some() && list.front().map(|log| log.start()).as_deref() != first {
            list.pop_front();
        }

        let len_removed = list.len();
        let num_removed = len_old - len_removed;
        let num_added = log.len() - list.len();

        log[list.len()..].iter().for_each(|log| {
            list.push_back(model::HealthCheckLog::from(log));
        });

        drop(list);

        self.items_changed(0, num_removed as u32, 0);
        self.items_changed(len_removed as u32, 0, num_added as u32);
    }
}
