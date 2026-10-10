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

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerPropertiesGroup)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_properties_group.ui")]
    pub(crate) struct ContainerPropertiesGroup {
        #[property(get, set, construct, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) id_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) created_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) size_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) restart_policy_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) health_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) health_status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) health_row_suffix_image: TemplateChild<gtk::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerPropertiesGroup {
        const NAME: &'static str = "PdsContainerPropertiesGroup";
        type Type = super::ContainerPropertiesGroup;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerPropertiesGroup {
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

            let ticks_expr = Self::Type::this_expression("root")
                .chain_property::<gtk::Window>("application")
                .chain_property::<crate::Application>("ticks");
            let container_expr = Self::Type::this_expression("container");
            let container_details_expr =
                container_expr.chain_property::<model::Container>("details");
            let restart_policy_expr =
                container_details_expr.chain_property::<model::ContainerDetails>("restart-policy");
            let health_status_expr =
                container_expr.chain_property::<model::Container>("health_status");
            let is_health_check_configured_expr =
                health_status_expr.chain_closure::<bool>(closure!(
                    |_: Self::Type, health_status: model::ContainerHealthStatus| health_status
                        != model::ContainerHealthStatus::Unconfigured
                ));

            container_expr
                .chain_property::<model::Container>("id")
                .chain_closure::<String>(closure!(
                    |_: Self::Type, id: &str| utils::format_id(id).to_owned()
                ))
                .bind(&*self.id_row, "subtitle", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [
                    &ticks_expr,
                    &container_expr.chain_property::<model::Container>("created"),
                ],
                closure!(|_: Self::Type, _ticks: u64, created: i64| {
                    utils::format_ago(utils::timespan_now(created))
                }),
            )
            .bind(&*self.created_row, "subtitle", Some(obj));

            container_details_expr
                .chain_property::<model::ContainerDetails>("size")
                .chain_closure::<String>(closure!(|_: Self::Type, size: i64| glib::format_size(
                    size as u64
                )))
                .bind(&*self.size_row, "subtitle", Some(obj));

            restart_policy_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, restart_policy: model::ContainerRestartPolicy| {
                        use model::ContainerRestartPolicy::*;

                        match restart_policy {
                            Always => gettext("Always"),
                            No => gettext("None"),
                            OnFailure => gettext("On failure"),
                            UnlessStopped => gettext("Unless stopped"),
                        }
                    }
                ))
                .bind(&*self.restart_policy_row, "subtitle", Some(obj));

            is_health_check_configured_expr.bind(&*self.health_row, "activatable", Some(obj));

            health_status_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, status: model::ContainerHealthStatus| status.to_string()
                ))
                .bind(&*self.health_status_label, "label", Some(obj));

            let css_classes = utils::css_classes(&*self.health_status_label);
            health_status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::ContainerHealthStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(String::from(
                                view::container::container_health_status_css_class(status),
                            )))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.health_status_label, "css-classes", Some(obj));

            is_health_check_configured_expr.bind(
                &*self.health_row_suffix_image,
                "visible",
                Some(obj),
            );
        }
    }

    impl WidgetImpl for ContainerPropertiesGroup {}
    impl PreferencesGroupImpl for ContainerPropertiesGroup {}
}

glib::wrapper! {
    pub(crate) struct ContainerPropertiesGroup(ObjectSubclass<imp::ContainerPropertiesGroup>)
        @extends gtk::Widget, adw::PreferencesGroup,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
