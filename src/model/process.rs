use std::cell::OnceCell;
use std::cell::RefCell;

use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::Properties;
use gtk::glib;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Process)]
    pub(crate) struct Process {
        #[property(get, set, construct_only, nullable)]
        pub(super) process_list: glib::WeakRef<model::ProcessList>,
        #[property(get, set, construct)]
        pub(super) user: RefCell<String>,
        #[property(get, set, construct_only)]
        pub(super) pid: OnceCell<i32>,
        #[property(get, set, construct)]
        pub(super) ppid: RefCell<i32>,
        #[property(get, set, construct)]
        pub(super) cpu: RefCell<f64>,
        #[property(get, set, construct)]
        pub(super) elapsed: RefCell<u64>,
        #[property(get, set, construct)]
        pub(super) tty: RefCell<String>,
        #[property(get, set, construct)]
        pub(super) time: RefCell<u64>,
        #[property(get, set, construct)]
        pub(super) command: RefCell<String>,
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
    pub(crate) fn new(process_list: &model::ProcessList, fields: &[impl AsRef<str>]) -> Process {
        glib::Object::builder()
            .property("process-list", process_list)
            .property("user", fields[0].as_ref())
            .property("pid", fields[1].as_ref().parse::<i32>().unwrap())
            .property("ppid", fields[2].as_ref().parse::<i32>().unwrap())
            .property("cpu", fields[3].as_ref().parse::<f64>().unwrap())
            .property("elapsed", parse_time(fields[4].as_ref()))
            .property("tty", fields[5].as_ref())
            .property("time", parse_time(fields[6].as_ref()))
            .property("command", fields[7].as_ref())
            .build()
    }

    pub(crate) fn update(&self, fields: &[impl AsRef<str>]) {
        self.set_user(fields[0].as_ref());
        self.set_ppid(fields[2].as_ref().parse::<i32>().unwrap());
        self.set_cpu(fields[3].as_ref().parse::<f64>().unwrap());
        self.set_elapsed(parse_time(fields[4].as_ref()));
        self.set_tty(fields[5].as_ref());
        self.set_time(parse_time(fields[6].as_ref()));
    }
}

fn parse_time(s: &str) -> u64 {
    match s.split_once("ms") {
        Some((millis, _)) => (millis.parse::<f64>().unwrap()).round() as u64,
        None => {
            let secs = s.split_once('s').unwrap().0;

            match secs.split_once('m') {
                Some((mins, secs)) => match mins.split_once('h') {
                    Some((hours, mins)) => {
                        hours.parse::<u64>().unwrap() * 3_600_000
                            + mins.parse::<u64>().unwrap() * 60_000
                            + (secs.parse::<f64>().unwrap() * 1_000.0) as u64
                    }
                    None => {
                        mins.parse::<u64>().unwrap() * 60_000
                            + (secs.parse::<f64>().unwrap() * 1_000.0) as u64
                    }
                },
                None => (secs.parse::<f64>().unwrap() * 1000.0) as u64,
            }
        }
    }
}
