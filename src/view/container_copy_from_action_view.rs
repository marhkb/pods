use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;
use crate::widget;

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerCopyFromActionView)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_copy_from_action_view.ui")]
    pub(crate) struct ContainerCopyFromActionActionView {
        #[property(get, set, construct_only, nullable)]
        pub(super) action: glib::WeakRef<model::ContainerCopyFromAction>,

        #[template_child]
        pub(super) action_indicator: TemplateChild<widget::ActionStateIndicator>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCopyFromActionActionView {
        const NAME: &'static str = "PdsContainerCopyFromActionView";
        type Type = super::ContainerCopyFromActionView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerCopyFromActionActionView {
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

            let obj = &*self.obj();

            let action_expr = Self::Type::this_expression("action");
            let action_ongoing_state_expr =
                action_expr.chain_property::<model::ContainerCopyFromAction>("state");

            action_ongoing_state_expr
                .chain_closure::<String>(closure!(|_: Self::Type, state: model::ActionState| {
                    match state {
                        model::ActionState::Ongoing => "ongoing",
                        model::ActionState::Cancelled => "cancelled",
                        model::ActionState::Failed => "failed",
                        model::ActionState::Finished => "finished",
                    }
                }))
                .bind(&*self.action_indicator, "action-state-name", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ContainerCopyFromActionActionView {}

    #[gtk::template_callbacks]
    impl ContainerCopyFromActionActionView {
        #[template_callback]
        fn format_size(&self, bytes: u64) -> glib::GString {
            glib::format_size(bytes)
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerCopyFromActionView(ObjectSubclass<imp::ContainerCopyFromActionActionView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::ContainerCopyFromAction> for view::ActionDialog {
    fn from(value: &model::ContainerCopyFromAction) -> Self {
        Self::new(
            value.upcast_ref(),
            &gettext("Copy From Container"),
            value
                .container()
                .map(|container| container.name())
                .as_deref(),
            &glib::Object::builder::<ContainerCopyFromActionView>()
                .property("action", value)
                .build(),
            250,
        )
    }
}
