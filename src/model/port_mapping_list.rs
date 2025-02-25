use std::cell::OnceCell;
use std::sync::OnceLock;

use gio::prelude::*;
use gio::subclass::prelude::*;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::podman;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct PortMappingList {
        pub(super) list: OnceCell<Vec<model::PortMapping>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PortMappingList {
        const NAME: &'static str = "PortMappingList";
        type Type = super::PortMappingList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for PortMappingList {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecUInt::builder("len")
                        .explicit_notify()
                        .build(),
                ]
            })
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "len" => self.obj().len().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj()
                .connect_items_changed(|obj, _, _, _| obj.notify("len"));
        }
    }

    impl ListModelImpl for PortMappingList {
        fn item_type(&self) -> glib::Type {
            model::PortMapping::static_type()
        }

        fn n_items(&self) -> u32 {
            self.obj().len()
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.obj().get(position as usize).map(|obj| obj.upcast())
        }
    }
}

glib::wrapper! {
    pub(crate) struct PortMappingList(ObjectSubclass<imp::PortMappingList>)
        @implements gio::ListModel;
}

impl From<Vec<podman::models::PortMapping>> for PortMappingList {
    fn from(port_mappings: Vec<podman::models::PortMapping>) -> Self {
        let obj: Self = glib::Object::builder().build();
        obj.imp()
            .list
            .set(
                port_mappings
                    .into_iter()
                    .map(model::PortMapping::from)
                    .collect(),
            )
            .unwrap();
        obj
    }
}

impl PortMappingList {
    pub(crate) fn get(&self, index: usize) -> Option<model::PortMapping> {
        self.imp().list.get().unwrap().get(index).cloned()
    }

    pub(crate) fn len(&self) -> u32 {
        self.imp().list.get().unwrap().len() as u32
    }
}
