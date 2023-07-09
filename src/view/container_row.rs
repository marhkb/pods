use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::model::prelude::*;
use crate::utils;
use crate::view;
use crate::widget;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_row.ui")]
    pub(crate) struct ContainerRow {
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[property(get, set = Self::set_container, construct, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) spinner: TemplateChild<widget::Spinner>,
        #[template_child]
        pub(super) check_button_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) pod_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) port_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) repo_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) health_status_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) stats_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) cpu_bar: TemplateChild<widget::CircularProgressBar>,
        #[template_child]
        pub(super) mem_bar: TemplateChild<widget::CircularProgressBar>,
        #[template_child]
        pub(super) end_box_revealer: TemplateChild<gtk::Revealer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerRow {
        const NAME: &'static str = "PdsContainerRow";
        type Type = super::ContainerRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("container-row.activate", None, |widget, _, _| {
                widget.activate();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerRow {
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

            let container_expr = Self::Type::this_expression("container");
            let container_list_expr =
                container_expr.chain_property::<model::Container>("container-list");

            let selection_mode_expr =
                container_list_expr.chain_property::<model::ContainerList>("selection-mode");

            selection_mode_expr.bind(&*self.check_button_revealer, "reveal-child", Some(obj));
            selection_mode_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, is_selection_mode: bool| {
                    !is_selection_mode
                }))
                .bind(&*self.end_box_revealer, "reveal-child", Some(obj));

            let status_expr = container_expr.chain_property::<model::Container>("status");
            let port_expr = container_expr.chain_property::<model::Container>("port");
            let health_status_expr =
                container_expr.chain_property::<model::Container>("health-status");
            let pod_expr = container_expr.chain_property::<model::Container>("pod");
            let stats_expr = container_expr.chain_property::<model::Container>("stats");

            container_expr
                .chain_property::<model::Container>("action-ongoing")
                .bind(&*self.spinner, "spinning", Some(obj));

            status_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| {
                        match status {
                            model::ContainerStatus::Running => "media-playback-start-symbolic",
                            model::ContainerStatus::Paused => "media-playback-pause-symbolic",
                            _ => "media-playback-stop-symbolic",
                        }
                    }
                ))
                .bind(&*self.spinner, "icon-name", Some(obj));

            let css_classes = utils::css_classes(self.spinner.upcast_ref());
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(String::from(
                                view::container::container_status_css_class(status),
                            )))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.spinner, "css-classes", Some(obj));

            gtk::ClosureExpression::new::<String>(
                &[
                    container_expr.chain_property::<model::Container>("name"),
                    container_expr.chain_property::<model::Container>("to-be-deleted"),
                ],
                closure!(|_: Self::Type, name: &str, to_be_deleted: bool| {
                    let name = utils::escape(name);
                    if to_be_deleted {
                        format!("<s>{name}</s>")
                    } else {
                        name
                    }
                }),
            )
            .bind(&*self.name_label, "label", Some(obj));

            pod_expr
                .chain_closure::<bool>(closure!(
                    |_: Self::Type, pod: Option<model::Pod>| pod.is_some()
                ))
                .bind(&*self.pod_image, "visible", Some(obj));

            port_expr.bind(&*self.port_label, "label", Some(obj));
            port_expr
                .chain_closure::<bool>(closure!(
                    |_: Self::Type, port: Option<String>| port.is_some()
                ))
                .bind(&*self.port_label, "visible", Some(obj));

            let css_classes = utils::css_classes(self.port_label.upcast_ref());
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(String::from(match status {
                                model::ContainerStatus::Running => "accent",
                                _ => "dim-label",
                            })))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.port_label, "css-classes", Some(obj));

            container_expr
                .chain_property::<model::Container>("image-name")
                .chain_closure::<String>(closure!(|_: Self::Type, name: Option<String>| {
                    utils::escape(&utils::format_option(name))
                }))
                .bind(&*self.repo_label, "label", Some(obj));

            health_status_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, status: model::ContainerHealthStatus| match status {
                        model::ContainerHealthStatus::Starting => "image-loading-symbolic",
                        model::ContainerHealthStatus::Healthy => "emblem-ok-symbolic",
                        model::ContainerHealthStatus::Unhealthy => "error-symbolic",
                        _ => "dialog-question-symbolic",
                    }
                ))
                .bind(&*self.health_status_image, "icon-name", Some(obj));

            gtk::ClosureExpression::new::<bool>(
                [status_expr.upcast_ref(), health_status_expr.upcast_ref()],
                closure!(|_: Self::Type,
                          status: model::ContainerStatus,
                          health_status: model::ContainerHealthStatus| {
                    status == model::ContainerStatus::Running
                        && health_status != model::ContainerHealthStatus::Unconfigured
                }),
            )
            .bind(&*self.health_status_image, "visible", Some(obj));

            let css_classes = utils::css_classes(self.health_status_image.upcast_ref());
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
                .bind(&*self.health_status_image, "css-classes", Some(obj));

            status_expr
                .chain_closure::<bool>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| matches!(
                        status,
                        model::ContainerStatus::Running
                    )
                ))
                .bind(&*self.stats_box, "visible", Some(obj));

            obj.bind_stats_percentage(
                gtk::ClosureExpression::new::<f64>(
                    [
                        container_list_expr
                            .chain_property::<model::ContainerList>("client")
                            .chain_property::<model::Client>("cpus")
                            .upcast_ref(),
                        stats_expr.upcast_ref(),
                    ],
                    closure!(
                        |_: Self::Type, cpus: i64, stats: Option<model::BoxedContainerStats>| {
                            if cpus > 0 {
                                stats
                                    .and_then(|stats| stats.cpu.map(|cpu| cpu / cpus as f64))
                                    .unwrap_or_default()
                            } else {
                                0.0
                            }
                        }
                    ),
                )
                .upcast_ref(),
                &self.cpu_bar,
            );
            obj.bind_stats_percentage(
                stats_expr
                    .chain_closure::<f64>(closure!(
                        |_: Self::Type, stats: Option<model::BoxedContainerStats>| stats
                            .and_then(|stats| stats.mem_perc)
                            .unwrap_or(0.0)
                    ))
                    .upcast_ref(),
                &self.mem_bar,
            );
        }
    }

    impl WidgetImpl for ContainerRow {}
    impl ListBoxRowImpl for ContainerRow {}

    impl ContainerRow {
        pub(super) fn set_container(&self, value: Option<&model::Container>) {
            let obj = &*self.obj();
            if obj.container().as_ref() == value {
                return;
            }

            let mut bindings = self.bindings.borrow_mut();
            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }

            if let Some(container) = value {
                let binding = container
                    .bind_property("selected", &*self.check_button, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();

                bindings.push(binding);
            }

            self.container.set(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerRow(ObjectSubclass<imp::ContainerRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for ContainerRow {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder()
            .property("container", container)
            .build()
    }
}

impl ContainerRow {
    fn bind_stats_percentage(
        &self,
        stats_expr: &gtk::Expression,
        progress_bar: &widget::CircularProgressBar,
    ) {
        let perc_expr =
            stats_expr.chain_closure::<f64>(closure!(|_: Self, value: f64| value * 0.01));

        let target = adw::PropertyAnimationTarget::new(progress_bar, "percentage");
        let animation = adw::TimedAnimation::builder()
            .widget(progress_bar)
            .duration(750)
            .target(&target)
            .build();

        perc_expr.watch(
            Some(self),
            clone!(@weak self as obj, @weak progress_bar, @strong perc_expr => move || {
                animation.set_value_from(progress_bar.percentage());
                animation.set_value_to(perc_expr.evaluate_as(Some(&obj)).unwrap_or(0.0));
                animation.play();
            }),
        );
    }

    fn activate(&self) {
        if let Some(container) = self.container().as_ref() {
            if container
                .container_list()
                .map(|list| list.is_selection_mode())
                .unwrap_or(false)
            {
                container.select();
            } else {
                let nav_page = adw::NavigationPage::builder()
                    .child(&view::ContainerDetailsPage::from(container))
                    .build();

                Self::this_expression("container")
                    .chain_property::<model::Container>("name")
                    .chain_closure::<String>(closure!(|_: Self, name: &str| gettext!(
                        "Container {}",
                        name
                    )))
                    .bind(&nav_page, "title", Some(self));

                utils::navigation_view(self.upcast_ref()).push(&nav_page);
            }
        }
    }
}
