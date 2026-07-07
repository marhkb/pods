use std::marker::PhantomData;

use adw::subclass::prelude::*;
use glib::Properties;
use glib::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ActionStateIndicator)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/action_state_indicator.ui")]
    pub(crate) struct ActionStateIndicator {
        #[property(get = Self::action_name, set = Self::set_action_name, explicit_notify)]
        pub(super) _action_state_name: PhantomData<String>,

        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ActionStateIndicator {
        const NAME: &'static str = "PdsActionStateIndicator";
        type Type = super::ActionStateIndicator;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ActionStateIndicator {
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

    impl WidgetImpl for ActionStateIndicator {}

    #[gtk::template_callbacks]
    impl ActionStateIndicator {
        fn action_name(&self) -> String {
            self.stack
                .visible_child_name()
                .map(Into::into)
                .unwrap_or_else(|| "ongoing".to_string())
        }

        fn set_action_name(&self, value: &str) {
            self.stack.set_visible_child_name(value);
        }

        #[template_callback]
        fn on_notify_stack_visible_child_name(&self) {
            self.obj().notify_action_state_name();
        }
    }
}

glib::wrapper! {
    pub(crate) struct ActionStateIndicator(ObjectSubclass<imp::ActionStateIndicator>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
