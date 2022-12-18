use std::cell::RefCell;
use std::collections::VecDeque;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::model;
use crate::podman;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct HealthCheckLogList {
        pub(super) list: RefCell<VecDeque<model::HealthCheckLog>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HealthCheckLogList {
        const NAME: &'static str = "HealthCheckLogList";
        type Type = super::HealthCheckLogList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for HealthCheckLogList {}

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

impl Default for HealthCheckLogList {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl HealthCheckLogList {
    pub(crate) fn sync(&self, logs: Vec<podman::models::HealthCheckLog>) {
        let mut list = self.imp().list.borrow_mut();

        let len_old = list.len();

        let first = logs.first().and_then(|log| log.start.as_deref());
        while list.front().is_some() && list.front().map(|log| log.start()) != first {
            list.pop_front();
        }

        let len_removed = list.len();
        let num_removed = len_old - len_removed;
        let num_added = logs.len() - list.len();

        logs[list.len()..].iter().for_each(|log| {
            list.push_back(model::HealthCheckLog::from(log));
        });

        drop(list);

        self.items_changed(0, num_removed as u32, 0);
        self.items_changed(len_removed as u32, 0, num_added as u32);
    }
}
