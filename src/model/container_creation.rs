use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::panic;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::podman;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ContainerCreation)]
    pub(crate) struct ContainerCreation {
        #[property(get = Self::client, set, construct)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, construct, nullable)]
        pub(super) image: glib::WeakRef<model::Image>,
        #[property(get, set = Self::set_pod, construct, nullable, explicit_notify)]
        pub(super) pod: glib::WeakRef<model::Pod>,
        #[property(get, set, construct, nullable)]
        pub(super) volume: glib::WeakRef<model::Volume>,

        #[property(get = Self::cmd_args)]
        pub(super) cmd_args: OnceCell<gio::ListStore>,
        #[property(get = Self::port_mappings)]
        pub(super) port_mappings: OnceCell<gio::ListStore>,
        #[property(get = Self::volumes)]
        pub(super) volumes: OnceCell<gio::ListStore>,
        #[property(get = Self::env_vars)]
        pub(super) env_vars: OnceCell<gio::ListStore>,
        #[property(get = Self::labels)]
        pub(super) labels: OnceCell<gio::ListStore>,

        #[property(get, set)]
        pub(super) name: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCreation {
        const NAME: &'static str = "ContainerCreation";
        type Type = super::ContainerCreation;
    }

    impl ObjectImpl for ContainerCreation {
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

    impl ContainerCreation {
        pub(super) fn client(&self) -> Option<model::Client> {
            self.client
                .upgrade()
                .or_else(|| {
                    self.obj()
                        .image()
                        .as_ref()
                        .and_then(model::Image::image_list)
                        .as_ref()
                        .and_then(model::ImageList::client)
                })
                .or_else(|| {
                    self.obj()
                        .pod()
                        .as_ref()
                        .and_then(model::Pod::pod_list)
                        .as_ref()
                        .and_then(model::PodList::client)
                })
                .or_else(|| {
                    self.obj()
                        .volume()
                        .and_then(|volume| volume.volume_list())
                        .and_then(|list| list.client())
                })
        }

        pub(super) fn cmd_args(&self) -> gio::ListStore {
            self.cmd_args
                .get_or_init(gio::ListStore::new::<model::Value>)
                .to_owned()
        }

        pub(super) fn port_mappings(&self) -> gio::ListStore {
            self.port_mappings
                .get_or_init(gio::ListStore::new::<model::PortMapping>)
                .to_owned()
        }

        pub(super) fn volumes(&self) -> gio::ListStore {
            self.volumes
                .get_or_init(gio::ListStore::new::<model::Mount>)
                .to_owned()
        }

        pub(super) fn env_vars(&self) -> gio::ListStore {
            self.env_vars
                .get_or_init(gio::ListStore::new::<model::KeyVal>)
                .to_owned()
        }

        pub(super) fn labels(&self) -> gio::ListStore {
            self.labels
                .get_or_init(gio::ListStore::new::<model::KeyVal>)
                .to_owned()
        }

        pub(super) fn set_pod(&self, value: Option<&model::Pod>) {
            let obj = &*self.obj();

            // obj.action_set_enabled(ACTION_CLEAR_POD, value.is_some());

            if obj.pod().as_ref() == value {
                return;
            }

            self.pod.set(value);
            obj.notify_pod();
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerCreation(ObjectSubclass<imp::ContainerCreation>);
}

impl From<&model::Client> for ContainerCreation {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl From<&model::Image> for ContainerCreation {
    fn from(image: &model::Image) -> Self {
        glib::Object::builder().property("image", image).build()
    }
}

impl From<&model::Pod> for ContainerCreation {
    fn from(pod: &model::Pod) -> Self {
        glib::Object::builder().property("pod", pod).build()
    }
}

impl From<&model::Volume> for ContainerCreation {
    fn from(volume: &model::Volume) -> Self {
        glib::Object::builder().property("volume", volume).build()
    }
}

impl ContainerCreation {
    pub(crate) fn new(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }

    pub(crate) fn create_container(&self, run: bool) -> model::Action {
        let name = self.name();

        self.client()
            .unwrap()
            .action_list()
            .create_container(self, run)
    }
}
