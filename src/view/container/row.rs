use std::cell::RefCell;

use adw::traits::AnimationExt;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::closure_local;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::model::SelectableExt;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container/row.ui")]
    pub(crate) struct Row {
        pub(super) container: glib::WeakRef<model::Container>,
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[template_child]
        pub(super) spinner: TemplateChild<view::Spinner>,
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
        pub(super) health_status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) stats_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) cpu_bar: TemplateChild<view::CircularProgressBar>,
        #[template_child]
        pub(super) mem_bar: TemplateChild<view::CircularProgressBar>,
        #[template_child]
        pub(super) end_box_revealer: TemplateChild<gtk::Revealer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsContainerRow";
        type Type = super::Row;
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

    impl ObjectImpl for Row {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Container>("container")
                        .explicit_notify()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "container" => self.obj().set_container(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container" => self.obj().container().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let container_expr = Self::Type::this_expression("container");

            let selection_mode_expr = container_expr
                .chain_property::<model::Container>("container-list")
                .chain_property::<model::ContainerList>("selection-mode");

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

            let css_classes = self.spinner.css_classes();
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(glib::GString::from(
                                super::super::container_status_css_class(status),
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

            let css_classes = self.port_label.css_classes();
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(glib::GString::from(match status {
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
                    |_: Self::Type, status: model::ContainerHealthStatus| status.to_string()
                ))
                .bind(&*self.health_status_label, "label", Some(obj));

            gtk::ClosureExpression::new::<bool>(
                [status_expr.upcast_ref(), health_status_expr.upcast_ref()],
                closure!(|_: Self::Type,
                          status: model::ContainerStatus,
                          health_status: model::ContainerHealthStatus| {
                    status == model::ContainerStatus::Running
                        && health_status != model::ContainerHealthStatus::Unconfigured
                }),
            )
            .bind(&*self.health_status_label, "visible", Some(obj));

            let css_classes = self.health_status_label.css_classes();
            health_status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::ContainerHealthStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(glib::GString::from(
                                super::super::container_health_status_css_class(status),
                            )))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.health_status_label, "css-classes", Some(obj));

            status_expr
                .chain_closure::<bool>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| matches!(
                        status,
                        model::ContainerStatus::Running
                    )
                ))
                .bind(&*self.stats_box, "visible", Some(obj));

            obj.bind_stats_percentage(stats_expr.upcast_ref(), |stats| stats.cpu, &self.cpu_bar);
            obj.bind_stats_percentage(
                stats_expr.upcast_ref(),
                |stats| stats.mem_perc,
                &self.mem_bar,
            );
        }
    }

    impl WidgetImpl for Row {}
    impl ListBoxRowImpl for Row {}
}

glib::wrapper! {
    pub(crate) struct Row(ObjectSubclass<imp::Row>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for Row {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder()
            .property("container", container)
            .build()
    }
}

impl Row {
    pub(crate) fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    fn set_container(&self, value: Option<&model::Container>) {
        if self.container().as_ref() == value {
            return;
        }

        let imp = self.imp();

        let mut bindings = imp.bindings.borrow_mut();
        while let Some(binding) = bindings.pop() {
            binding.unbind();
        }

        if let Some(container) = value {
            let binding = container
                .bind_property("selected", &*imp.check_button, "active")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            bindings.push(binding);
        }

        imp.container.set(value);
        self.notify("container");
    }

    fn bind_stats_percentage<F>(
        &self,
        stats_expr: &gtk::Expression,
        fraction_op: F,
        progress_bar: &view::CircularProgressBar,
    ) where
        F: Fn(model::BoxedContainerStats) -> Option<f64> + Clone + 'static,
    {
        #[rustfmt::skip]
        let perc_expr = stats_expr.chain_closure::<f64>(
            closure_local!(|_: Self, stats: Option<model::BoxedContainerStats>| {
                stats
                    .and_then(|stats| fraction_op(stats).map(|perc| perc * 0.01))
                    .unwrap_or_default()
            })
        );

        let target = adw::PropertyAnimationTarget::new(progress_bar, "percentage");
        let animation = adw::TimedAnimation::builder()
            .widget(progress_bar)
            .duration(750)
            .target(&target)
            .build();

        stats_expr.watch(
            Some(self),
            clone!(@weak self as obj, @weak progress_bar => move || {
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
                utils::find_leaflet_overlay(self.upcast_ref())
                    .show_details(view::ContainerDetailsPage::from(container).upcast_ref());
            }
        }
    }
}
