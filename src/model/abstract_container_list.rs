use gtk::gio;
use gtk::glib;
use gtk::glib::subclass::Signal;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;

use crate::model;

mod imp {
    use super::*;

    #[derive(Copy, Clone, Debug)]
    pub(crate) struct AbstractContainerList(glib::gobject_ffi::GTypeInterface);

    #[glib::object_interface]
    unsafe impl ObjectInterface for AbstractContainerList {
        const NAME: &'static str = "AbstractContainerList";
        type Prerequisites = (gio::ListModel,);

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder(
                        "container-added",
                        &[model::Container::static_type().into()],
                        <()>::static_type().into(),
                    )
                    .build(),
                    Signal::builder(
                        "container-name-changed",
                        &[model::Container::static_type().into()],
                        <()>::static_type().into(),
                    )
                    .build(),
                    Signal::builder(
                        "container-removed",
                        &[model::Container::static_type().into()],
                        <()>::static_type().into(),
                    )
                    .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecUInt::new(
                        "len",
                        "Len",
                        "The length of this list",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt::new(
                        "created",
                        "Created",
                        "The number of created containers",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt::new(
                        "dead",
                        "Dead",
                        "The number of dead containers",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt::new(
                        "exited",
                        "Exited",
                        "The number of exited containers",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt::new(
                        "paused",
                        "Paused",
                        "The number of paused containers",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READABLE,
                    ),
                    glib::ParamSpecUInt::new(
                        "running",
                        "Running",
                        "The number of running containers",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READABLE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }
    }
}

glib::wrapper! {
    pub(crate) struct AbstractContainerList(ObjectInterface<imp::AbstractContainerList>)
        @requires gio::ListModel;
}

pub(crate) trait AbstractContainerListExt: IsA<AbstractContainerList> {
    fn container_added(&self, container: &model::Container) {
        self.emit_by_name::<()>("container-added", &[container]);
    }

    fn container_name_changed(&self, container: &model::Container) {
        self.emit_by_name::<()>("container-name-changed", &[container]);
    }

    fn container_removed(&self, model: &model::Container) {
        self.emit_by_name::<()>("container-removed", &[&model]);
    }

    fn connect_container_added<F: Fn(&Self, &model::Container) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("container-added", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let container = values[1].get::<model::Container>().unwrap();
            f(&obj, &container);

            None
        })
    }

    fn connect_container_name_changed<F: Fn(&Self, &model::Container) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("container-name-changed", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let container = values[1].get::<model::Container>().unwrap();
            f(&obj, &container);

            None
        })
    }

    fn connect_container_removed<F: Fn(&Self, &model::Container) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("container-removed", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let container = values[1].get::<model::Container>().unwrap();
            f(&obj, &container);

            None
        })
    }
}

impl<T: IsA<AbstractContainerList>> AbstractContainerListExt for T {}

unsafe impl<T: ObjectSubclass> IsImplementable<T> for AbstractContainerList {}
