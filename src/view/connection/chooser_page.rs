use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/connection/chooser-page.ui")]
    pub(crate) struct ChooserPage {
        pub(super) connection_manager: glib::WeakRef<model::ConnectionManager>,
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ChooserPage {
        const NAME: &'static str = "PdsConnectionChooserPage";
        type Type = super::ChooserPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.set_css_name("connectionchooserpage");

            klass.install_action(
                "connection-chooser-page.add-connection",
                None,
                |widget, _, _| {
                    widget.add_connection_creation_page();
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ChooserPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::ConnectionManager>(
                    "connection-manager",
                )
                .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
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

    impl WidgetImpl for ChooserPage {}
}

glib::wrapper! {
    pub(crate) struct ChooserPage(ObjectSubclass<imp::ChooserPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ChooserPage {
    pub(crate) fn connection_manager(&self) -> Option<model::ConnectionManager> {
        self.imp().connection_manager.upgrade()
    }

    pub(crate) fn set_connection_manager(&self, value: Option<&model::ConnectionManager>) {
        if self.connection_manager().as_ref() == value {
            return;
        }

        self.imp().connection_manager.set(value);
        self.notify("connection-manager");
    }

    fn add_connection_creation_page(&self) {
        if let Some(connection_manager) = self.connection_manager() {
            self.imp()
                .leaflet_overlay
                .show_details(&view::ConnectionCreationPage::from(&connection_manager));
        }
    }
}
