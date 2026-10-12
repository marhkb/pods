use std::cell::Cell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::Spinner)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/spinner.ui")]
    pub(crate) struct Spinner {
        #[property(get, set, nullable)]
        pub(super) icon_name: RefCell<Option<String>>,
        #[property(get, set = Self::set_spinning, explicit_notify)]
        pub(super) spinning: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Spinner {
        const NAME: &'static str = "PdsSpinner";
        type Type = super::Spinner;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Spinner {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for Spinner {}

    impl Spinner {
        fn set_spinning(&self, value: bool) {
            let obj = &*self.obj();
            if obj.spinning() == value {
                return;
            }

            self.spinning.set(value);
            obj.notify_spinning();
        }
    }
}

glib::wrapper! {
    pub(crate) struct Spinner(ObjectSubclass<imp::Spinner>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
