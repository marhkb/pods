use gtk::glib;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/connection/chooser-page.ui")]
    pub(crate) struct ChooserPage(pub(super) WeakRef<model::ConnectionManager>);

    #[glib::object_subclass]
    impl ObjectSubclass for ChooserPage {
        const NAME: &'static str = "PdsConnectionChooserPage";
        type Type = super::ChooserPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.set_css_name("connectionchooserpage");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ChooserPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "connection-manager",
                    "Connection Manager",
                    "The connection manager client",
                    model::ConnectionManager::static_type(),
                    glib::ParamFlags::READWRITE,
                )]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "connection-manager" => obj.set_connection_manager(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "connection-manager" => obj.connection_manager().to_value(),
                _ => unimplemented!(),
            }
        }

        fn dispose(&self, obj: &Self::Type) {
            let mut child = obj.first_child();
            while let Some(child_) = child {
                child = child_.next_sibling();
                child_.unparent();
            }
        }
    }

    impl WidgetImpl for ChooserPage {}
}

glib::wrapper! {
    pub(crate) struct ChooserPage(ObjectSubclass<imp::ChooserPage>)
        @extends gtk::Widget;
}

impl ChooserPage {
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