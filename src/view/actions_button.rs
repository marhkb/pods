use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ActionsButton)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/actions_button.ui")]
    pub(crate) struct ActionsButton {
        #[property(get, set)]
        pub(super) action_list: glib::WeakRef<model::ActionList>,
        #[template_child]
        pub(super) button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ActionsButton {
        const NAME: &'static str = "PdsActionsButton";
        type Type = super::ActionsButton;
        type ParentType = gtk::Widget;
        type Interfaces = (gtk::Actionable,);

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.set_css_name("actionsbutton");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ActionsButton {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                Self::derived_properties()
                    .iter()
                    .cloned()
                    .chain(vec![
                        glib::ParamSpecString::builder("action-name")
                            .explicit_notify()
                            .build(),
                        glib::ParamSpecVariant::builder("action-target", glib::VariantTy::ANY)
                            .explicit_notify()
                            .build(),
                    ])
                    .collect::<Vec<_>>()
            })
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "action-name" => self.set_action_name(value.get().unwrap()),
                "action-target" => self.set_action_target_value(
                    value.get::<Option<glib::Variant>>().unwrap().as_ref(),
                ),
                _ => self.derived_set_property(id, value, pspec),
            }
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "action-name" => self.action_name().to_value(),
                "action-target" => self.action_name().to_value(),
                _ => self.derived_property(id, pspec),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let action_list_expr = Self::Type::this_expression("action-list");

            gtk::ClosureExpression::new::<Vec<String>>(
                &[
                    action_list_expr.chain_property::<model::ActionList>("failed"),
                    action_list_expr.chain_property::<model::ActionList>("cancelled"),
                    action_list_expr.chain_property::<model::ActionList>("ongoing"),
                ],
                closure!(|_: Self::Type, failed: u32, cancelled: u32, ongoing: u32| {
                    vec![if failed > 0 {
                        "failed"
                    } else if cancelled > 0 {
                        "cancelled"
                    } else if ongoing > 0 {
                        "good"
                    } else {
                        "finished"
                    }
                    .to_string()]
                }),
            )
            .bind(obj, "css-classes", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ActionsButton {}

    impl ActionableImpl for ActionsButton {
        fn action_name(&self) -> Option<glib::GString> {
            self.button.action_name()
        }

        fn action_target_value(&self) -> Option<glib::Variant> {
            self.button.action_target_value()
        }

        fn set_action_name(&self, name: Option<&str>) {
            self.button.set_action_name(name);
        }

        fn set_action_target_value(&self, value: Option<&glib::Variant>) {
            self.button.set_action_target_value(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct ActionsButton(ObjectSubclass<imp::ActionsButton>)
        @extends gtk::Widget,
        @implements gtk::Actionable, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
