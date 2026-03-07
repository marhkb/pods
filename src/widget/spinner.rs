use std::cell::Cell;
use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/spinner.ui")]
    pub(crate) struct Spinner {
        pub(super) spinning: Cell<bool>,
        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Spinner {
        const NAME: &'static str = "PdsSpinner";
        type Type = super::Spinner;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Spinner {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecString::builder("icon-name")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecBoolean::builder("spinning")
                        .explicit_notify()
                        .build(),
                ]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "icon-name" => obj.set_icon_name(value.get().unwrap_or_default()),
                "spinning" => obj.set_spinning(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "icon-name" => obj.icon_name().to_value(),
                "spinning" => obj.is_spinning().to_value(),
                _ => unimplemented!(),
            }
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for Spinner {}

    #[gtk::template_callbacks]
    impl Spinner {
        #[template_callback]
        fn on_image_notify_icon_name(&self) {
            self.obj().notify("icon-name");
        }
    }
}

glib::wrapper! {
    pub(crate) struct Spinner(ObjectSubclass<imp::Spinner>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Spinner {
    pub(crate) fn icon_name(&self) -> Option<glib::GString> {
        self.imp().image.icon_name()
    }

    pub(crate) fn set_icon_name(&self, value: Option<&str>) {
        self.imp().image.set_icon_name(value);
    }

    pub(crate) fn is_spinning(&self) -> bool {
        self.imp().spinning.get()
    }

    pub(crate) fn set_spinning(&self, value: bool) {
        if self.is_spinning() == value {
            return;
        }

        self.imp().spinning.set(value);
        self.notify("spinning");
    }
}
