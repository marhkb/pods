use std::cell::Cell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::expression::ExpressionExt;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ZoomControl)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/zoom_control.ui")]
    pub(crate) struct ZoomControl {
        #[property(get, set, minimum = 0.0)]
        pub(super) zoom_factor: Cell<f64>,
        #[property(get, set, nullable)]
        pub(super) zoom_in_action_name: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        pub(super) zoom_normal_action_name: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        pub(super) zoom_out_action_name: RefCell<Option<String>>,

        #[template_child]
        pub(super) zoom_normal_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ZoomControl {
        const NAME: &'static str = "PdsZoomControl";
        type Type = super::ZoomControl;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ZoomControl {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            Self::Type::this_expression("zoom-factor")
                .map(|factor: f64| format!("{:.0}%", 100.0 * factor))
                .bind(&*self.zoom_normal_button, "label", Some(&*self.obj()));
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ZoomControl {}
}

glib::wrapper! {
    pub(crate) struct ZoomControl(ObjectSubclass<imp::ZoomControl>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
