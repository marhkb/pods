use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/welcome-page.ui")]
    pub(crate) struct WelcomePage(pub(super) glib::WeakRef<model::ConnectionManager>);

    #[glib::object_subclass]
    impl ObjectSubclass for WelcomePage {
        const NAME: &'static str = "PdsWelcomePage";
        type Type = super::WelcomePage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WelcomePage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::ConnectionManager>(
                    "connection-manager",
                )
                .explicit_notify()
                .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "connection-manager" => self.obj().set_connection_manager(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "connection-manager" => self.obj().connection_manager().to_value(),
                _ => unimplemented!(),
            }
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for WelcomePage {}
}

glib::wrapper! {
    pub(crate) struct WelcomePage(ObjectSubclass<imp::WelcomePage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl WelcomePage {
    pub(crate) fn connection_manager(&self) -> Option<model::ConnectionManager> {
        self.imp().0.upgrade()
    }

    pub(crate) fn set_connection_manager(&self, value: Option<&model::ConnectionManager>) {
        if self.connection_manager().as_ref() == value {
            return;
        }
        self.imp().0.set(value);
        self.notify("connection-manager");
    }
}
