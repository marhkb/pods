use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;

use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::engine;
use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Process)]
    pub(crate) struct Process {
        #[property(get, set, construct_only, nullable)]
        pub(super) process_list: glib::WeakRef<model::ProcessList>,
        #[property(get, set = Self::set_user, explicit_notify, construct)]
        pub(super) user: RefCell<String>,
        #[property(get, set, construct_only)]
        pub(super) pid: OnceCell<i32>,
        #[property(get, set = Self::set_ppid, explicit_notify, construct)]
        pub(super) ppid: Cell<i32>,
        #[property(get, set = Self::set_cpu, explicit_notify, construct)]
        pub(super) cpu: Cell<f64>,
        #[property(get, set = Self::set_elapsed, explicit_notify, construct)]
        pub(super) elapsed: Cell<i64>,
        #[property(get, set = Self::set_tty, explicit_notify, construct)]
        pub(super) tty: RefCell<String>,
        #[property(get, set= Self::set_time, explicit_notify, construct)]
        pub(super) time: Cell<i64>,
        #[property(get, set, construct_only)]
        pub(super) command: RefCell<String>,
    }

    impl Process {
        fn set_user(&self, user: &str) {
            if *self.user.borrow() == user {
                return;
            }
            self.user.replace(user.to_owned());
            self.obj().notify_user();
        }

        fn set_ppid(&self, ppid: i32) {
            let obj = &*self.obj();
            if obj.ppid() == ppid {
                return;
            }
            self.ppid.set(ppid);
            obj.notify_ppid();
        }

        fn set_cpu(&self, cpu: f64) {
            let obj = &*self.obj();
            if obj.cpu() == cpu {
                return;
            }
            self.cpu.set(cpu);
            obj.notify_cpu();
        }

        fn set_elapsed(&self, elapsed: i64) {
            let obj = &*self.obj();
            if obj.elapsed() == elapsed {
                return;
            }
            self.elapsed.set(elapsed);
            obj.notify_elapsed();
        }

        fn set_tty(&self, tty: &str) {
            if *self.tty.borrow() == tty {
                return;
            }
            self.tty.replace(tty.to_owned());
            self.obj().notify_tty();
        }

        fn set_time(&self, time: i64) {
            let obj = &*self.obj();
            if obj.time() == time {
                return;
            }
            self.time.set(time);
            obj.notify_time();
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Process {
        const NAME: &'static str = "Process";
        type Type = super::Process;
    }

    impl ObjectImpl for Process {
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
}

glib::wrapper! {
    pub(crate) struct Process(ObjectSubclass<imp::Process>);
}

impl Process {
    pub(crate) fn new(
        process_list: &model::ProcessList,
        process: engine::dto::TopProcess,
    ) -> Process {
        glib::Object::builder()
            .property("process-list", process_list)
            .property("user", process.user)
            .property("pid", process.pid)
            .property("ppid", process.ppid)
            .property("cpu", process.cpu)
            .property("elapsed", process.elapsed)
            .property("tty", process.tty)
            .property("time", process.time)
            .property("command", process.command)
            .build()
    }

    pub(crate) fn update(&self, process: engine::dto::TopProcess) {
        self.set_user(process.user);
        self.set_ppid(process.ppid);
        self.set_cpu(process.cpu);
        self.set_elapsed(process.elapsed);
        self.set_tty(process.tty);
        self.set_time(process.time);
    }
}
