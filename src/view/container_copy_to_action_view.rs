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
    #[properties(wrapper_type = super::ContainerCopyToActionView)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_copy_to_action_view.ui")]
    pub(crate) struct ContainerCopyToActionActionView {
        #[property(get, set, construct_only, nullable)]
        pub(super) action: glib::WeakRef<model::ContainerCopyToAction>,

        #[template_child]
        pub(super) create_tar_action_indicator: TemplateChild<widget::ActionStateIndicator>,
        #[template_child]
        pub(super) unwrap_tar_action_indicator: TemplateChild<widget::ActionStateIndicator>,
        #[template_child]
        pub(super) copy_bytes_action_indicator: TemplateChild<widget::ActionStateIndicator>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCopyToActionActionView {
        const NAME: &'static str = "PdsContainerCopyToActionView";
        type Type = super::ContainerCopyToActionView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerCopyToActionActionView {
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
            let action_state_expr =
                action_expr.chain_property::<model::ContainerCopyToAction>("state");
            let action_ongoing_state_expr =
                action_expr.chain_property::<model::ContainerCopyToAction>("ongoing-state");

            gtk::ClosureExpression::new::<String>(
                [&action_state_expr, &action_ongoing_state_expr],
                closure!(
                    |_: Self::Type,
                     state: model::ActionState,
                     ongoing_state: model::ContainerCopyToActionOngoingState| {
                        action_state_name(
                            state,
                            ongoing_state,
                            model::ContainerCopyToActionOngoingState::CreateTar,
                        )
                    }
                ),
            )
            .bind(
                &*self.create_tar_action_indicator,
                "action-state-name",
                Some(obj),
            );

            gtk::ClosureExpression::new::<String>(
                [&action_state_expr, &action_ongoing_state_expr],
                closure!(
                    |_: Self::Type,
                     state: model::ActionState,
                     ongoing_state: model::ContainerCopyToActionOngoingState| {
                        action_state_name(
                            state,
                            ongoing_state,
                            model::ContainerCopyToActionOngoingState::UnwrapTar,
                        )
                    }
                ),
            )
            .bind(
                &*self.unwrap_tar_action_indicator,
                "action-state-name",
                Some(obj),
            );

            gtk::ClosureExpression::new::<String>(
                [&action_state_expr, &action_ongoing_state_expr],
                closure!(
                    |_: Self::Type,
                     state: model::ActionState,
                     ongoing_state: model::ContainerCopyToActionOngoingState| {
                        action_state_name(
                            state,
                            ongoing_state,
                            model::ContainerCopyToActionOngoingState::CopyBytes,
                        )
                    }
                ),
            )
            .bind(
                &*self.copy_bytes_action_indicator,
                "action-state-name",
                Some(obj),
            );
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ContainerCopyToActionActionView {}

    #[gtk::template_callbacks]
    impl ContainerCopyToActionActionView {
        #[template_callback]
        fn format_size(&self, bytes: u64) -> glib::GString {
            glib::format_size(bytes)
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerCopyToActionView(ObjectSubclass<imp::ContainerCopyToActionActionView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::ContainerCopyToAction> for view::ActionDialog {
    fn from(value: &model::ContainerCopyToAction) -> Self {
        Self::new(
            value.upcast_ref(),
            &gettext("Copy To Container"),
            value
                .container()
                .map(|container| container.name())
                .as_deref(),
            &glib::Object::builder::<ContainerCopyToActionView>()
                .property("action", value)
                .build(),
            250,
        )
    }
}

fn action_state_name(
    state: model::ActionState,
    ongoing_state: model::ContainerCopyToActionOngoingState,
    target_ongoing_state: model::ContainerCopyToActionOngoingState,
) -> &'static str {
    use model::ActionState::*;

    match state {
        Ongoing => {
            if ongoing_state < target_ongoing_state {
                "waiting"
            } else if ongoing_state > target_ongoing_state {
                "finished"
            } else {
                "ongoing"
            }
        }
        Cancelled if ongoing_state == target_ongoing_state => "cancelled",
        Failed if ongoing_state == target_ongoing_state => "failed",
        Finished => "finished",
        _ => {
            if ongoing_state > target_ongoing_state {
                "finished"
            } else {
                "waiting"
            }
        }
    }
}
