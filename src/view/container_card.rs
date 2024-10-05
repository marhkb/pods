use std::cell::Cell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::closure_local;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::model::prelude::*;
use crate::utils;
use crate::view;
use crate::widget;

const ACTION_RENAME: &str = "container-card.rename";
const ACTION_START_OR_RESUME: &str = "container-card.start";
const ACTION_STOP: &str = "container-card.stop";
const ACTION_KILL: &str = "container-card.kill";
const ACTION_RESTART: &str = "container-card.restart";
const ACTION_PAUSE: &str = "container-card.pause";
const ACTION_RESUME: &str = "container-card.resume";
const ACTION_DELETE: &str = "container-card.delete";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerCard)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_card.ui")]
    pub(crate) struct ContainerCard {
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[property(get, set, construct, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) spinner: TemplateChild<widget::Spinner>,
        #[template_child]
        pub(super) name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) repo_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) edit_select_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) selection_check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) resources_status_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) cpu_progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) memory_progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) network_down_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) network_down_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) network_up_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) network_up_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) disk_write_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) disk_write_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) disk_read_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) disk_read_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) ports_pod_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) ports_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub(super) pod_center_box: TemplateChild<gtk::CenterBox>,
        #[template_child]
        pub(super) pod_name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) action_center_box: TemplateChild<gtk::CenterBox>,
        #[template_child]
        pub(super) start_or_resume_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) stop_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) spinning_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCard {
        const NAME: &'static str = "PdsContainerCard";
        type Type = super::ContainerCard;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.set_css_name("containercard");

            klass.install_action("container-card.activate", None, |widget, _, _| {
                widget.activate();
            });

            klass.install_action(ACTION_RENAME, None, |widget, _, _| {
                view::container::rename(widget, widget.container().as_ref());
            });

            klass.install_action(ACTION_START_OR_RESUME, None, |widget, _, _| {
                if widget.container().map(|c| c.can_start()).unwrap_or(false) {
                    view::container::start(widget, widget.container());
                } else {
                    view::container::resume(widget, widget.container());
                }
            });
            klass.install_action(ACTION_STOP, None, |widget, _, _| {
                view::container::stop(widget, widget.container());
            });
            klass.install_action(ACTION_KILL, None, |widget, _, _| {
                view::container::kill(widget, widget.container());
            });
            klass.install_action(ACTION_RESTART, None, |widget, _, _| {
                view::container::restart(widget, widget.container());
            });
            klass.install_action(ACTION_PAUSE, None, |widget, _, _| {
                view::container::pause(widget, widget.container());
            });
            klass.install_action(ACTION_RESUME, None, |widget, _, _| {
                view::container::resume(widget, widget.container());
            });
            klass.install_action(ACTION_DELETE, None, |widget, _, _| {
                widget.delete();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerCard {
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
            let status_expr = container_expr.chain_property::<model::Container>("status");
            let container_list_expr =
                container_expr.chain_property::<model::Container>("container-list");
            let selection_mode_expr =
                container_list_expr.chain_property::<model::ContainerList>("selection-mode");

            selection_mode_expr
                .chain_closure::<String>(closure!(|_: Self::Type, is_selection_mode: bool| {
                    if is_selection_mode {
                        "select"
                    } else {
                        "edit"
                    }
                }))
                .bind(
                    &self.edit_select_stack.get(),
                    "visible-child-name",
                    Some(obj),
                );
            selection_mode_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, is_selection_mode: bool| {
                    !is_selection_mode
                }))
                .bind(&self.action_center_box.get(), "sensitive", Some(obj));

            let health_status_expr =
                container_expr.chain_property::<model::Container>("health-status");
            let pod_expr = container_expr.chain_property::<model::Container>("pod");
            let pod_name_expr = pod_expr.chain_property::<model::Pod>("name");
            let pod_status_expr = pod_expr.chain_property::<model::Pod>("status");
            let stats_expr = container_expr.chain_property::<model::Container>("stats");

            status_expr.watch(
                Some(obj),
                clone!(
                    #[weak]
                    obj,
                    move || obj.update_actions()
                ),
            );
            container_expr
                .chain_property::<model::Container>("action-ongoing")
                .watch(
                    Some(obj),
                    clone!(
                        #[weak]
                        obj,
                        move || obj.update_actions()
                    ),
                );

            let css_classes = utils::css_classes(obj);
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(if status == model::ContainerStatus::Running {
                                None
                            } else {
                                Some("not-running".to_string())
                            })
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(obj, "css-classes", Some(obj));

            container_expr
                .chain_property::<model::Container>("action-ongoing")
                .bind(&*self.spinner, "spinning", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [&status_expr, &health_status_expr],
                closure!(|_: Self::Type,
                          status: model::ContainerStatus,
                          health_status: model::ContainerHealthStatus| {
                    match status {
                        model::ContainerStatus::Running => match health_status {
                            model::ContainerHealthStatus::Healthy => "heart-filled-symbolic",
                            model::ContainerHealthStatus::Unhealthy => "heart-broken-symbolic",
                            _ => "media-playback-start-symbolic",
                        },
                        model::ContainerStatus::Paused => "media-playback-pause-symbolic",
                        _ => "media-playback-stop-symbolic",
                    }
                }),
            )
            .bind(&*self.spinner, "icon-name", Some(obj));

            let css_classes = utils::css_classes(&*self.spinner);
            gtk::ClosureExpression::new::<Vec<String>>(
                [&status_expr, &health_status_expr],
                closure!(|_: Self::Type,
                          status: model::ContainerStatus,
                          health_status: model::ContainerHealthStatus| {
                    css_classes
                        .iter()
                        .cloned()
                        .chain(Some(String::from(
                            view::container::container_status_combined_css_class(
                                status,
                                health_status,
                            ),
                        )))
                        .collect::<Vec<_>>()
                }),
            )
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

            container_expr
                .chain_property::<model::Container>("image-name")
                .chain_closure::<String>(closure!(|_: Self::Type, name: Option<String>| {
                    utils::escape(&utils::format_option(name))
                }))
                .bind(&*self.repo_label, "label", Some(obj));

            status_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| {
                        match status {
                            model::ContainerStatus::Running => "running",
                            _ => "not-running",
                        }
                    }
                ))
                .bind(
                    &*self.resources_status_stack,
                    "visible-child-name",
                    Some(obj),
                );

            #[rustfmt::skip]
            obj.bind_stats_fraction(
                gtk::ClosureExpression::new::<f64>(
                    [
                        container_list_expr
                            .chain_property::<model::ContainerList>("client")
                            .chain_property::<model::Client>("cpus")
                            .upcast_ref(),
                        stats_expr.upcast_ref(),
                    ],
                    closure!(|_: Self::Type, cpus: i64, stats: Option<model::BoxedContainerStats>| {
                        if cpus > 0 {
                            stats
                                .and_then(|stats| stats.cpu.map(|cpu| cpu / cpus as f64))
                                .unwrap_or_default()
                        } else {
                            0.0
                        }
                    }),
                )
                .upcast_ref(),
                &self.cpu_progress_bar,
            );

            obj.bind_stats_fraction(
                stats_expr
                    .chain_closure::<f64>(closure!(
                        |_: Self::Type, stats: Option<model::BoxedContainerStats>| stats
                            .and_then(|stats| stats.mem_perc)
                            .unwrap_or(0.0)
                    ))
                    .upcast_ref(),
                &self.memory_progress_bar,
            );

            obj.bind_stats_throughput(
                stats_expr
                    .chain_closure::<u64>(closure!(
                        |_: Self::Type, stats: Option<model::BoxedContainerStats>| {
                            stats.and_then(|stats| stats.net_input).unwrap_or(0)
                        }
                    ))
                    .upcast_ref(),
                &self.network_down_box,
                &self.network_down_label,
            );

            obj.bind_stats_throughput(
                stats_expr
                    .chain_closure::<u64>(closure!(
                        |_: Self::Type, stats: Option<model::BoxedContainerStats>| {
                            stats.and_then(|stats| stats.net_output).unwrap_or(0)
                        }
                    ))
                    .upcast_ref(),
                &self.network_up_box,
                &self.network_up_label,
            );

            obj.bind_stats_throughput(
                stats_expr
                    .chain_closure::<u64>(closure!(
                        |_: Self::Type, stats: Option<model::BoxedContainerStats>| {
                            stats.and_then(|stats| stats.block_input).unwrap_or(0)
                        }
                    ))
                    .upcast_ref(),
                &self.disk_read_box,
                &self.disk_read_label,
            );

            obj.bind_stats_throughput(
                stats_expr
                    .chain_closure::<u64>(closure!(
                        |_: Self::Type, stats: Option<model::BoxedContainerStats>| {
                            stats.and_then(|stats| stats.block_output).unwrap_or(0)
                        }
                    ))
                    .upcast_ref(),
                &self.disk_write_box,
                &self.disk_write_label,
            );

            status_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| status.to_string()
                ))
                .bind(&self.status_label.get(), "label", Some(obj));

            pod_name_expr.bind(&*self.pod_name_label, "label", Some(obj));

            let css_classes = utils::css_classes(&*self.pod_center_box);
            pod_status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::PodStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(String::from(view::pod::pod_status_css_class(status))))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&self.pod_center_box.get(), "css-classes", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ContainerCard {}

    #[gtk::template_callbacks]
    impl ContainerCard {
        #[template_callback]
        fn on_mouse_1_released(gesture_click: &gtk::GestureClick) {
            gesture_click.set_state(gtk::EventSequenceState::Claimed);
            gesture_click
                .widget()
                .unwrap()
                .downcast::<<Self as ObjectSubclass>::Type>()
                .unwrap()
                .activate();
        }

        #[template_callback]
        fn on_key_pressed(
            &self,
            key: gdk::Key,
            _: u32,
            _: gdk::ModifierType,
            _: &gtk::EventControllerKey,
        ) -> glib::ControlFlow {
            match key {
                gdk::Key::Return => {
                    self.obj().activate();
                    glib::ControlFlow::Continue
                }
                _ => glib::ControlFlow::Break,
            }
        }

        #[template_callback]
        fn on_notify_container(&self) {
            let mut bindings = self.bindings.borrow_mut();
            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }

            let obj = &*self.obj();

            if let Some(container) = obj.container() {
                let binding = container
                    .bind_property("selected", &*self.selection_check_button, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();

                bindings.push(binding);

                if !container.has_pod() {
                    if container.ports().len() > 0 {
                        self.ports_pod_stack.set_visible_child_name("ports");
                        self.ports_flow_box.bind_model(
                            Some(&container.ports()),
                            clone!(
                                #[weak]
                                obj,
                                #[upgrade_or_panic]
                                move |item| {
                                    let port_mapping =
                                        item.downcast_ref::<model::PortMapping>().unwrap();

                                    let label = gtk::Label::builder()
                                        .css_classes(["status-badge-small", "numeric"])
                                        .halign(gtk::Align::Center)
                                        .valign(gtk::Align::Center)
                                        .label(format!(
                                            "{}/{}",
                                            port_mapping.host_port(),
                                            port_mapping.protocol()
                                        ))
                                        .build();

                                    let css_classes = utils::css_classes(&label);
                                    super::ContainerCard::this_expression("container")
                                        .chain_property::<model::Container>("status")
                                        .chain_closure::<Vec<String>>(closure!(
                                        |_: super::ContainerCard, status: model::ContainerStatus| {
                                            css_classes
                                                .iter()
                                                .cloned()
                                                .chain(Some(String::from(
                                                    super::super::container_status_css_class(status)
                                                )))
                                                .collect::<Vec<_>>()
                                        }
                                    ))
                                        .bind(&label, "css-classes", Some(&obj));

                                    gtk::FlowBoxChild::builder()
                                        .halign(gtk::Align::Start)
                                        .child(&label)
                                        .build()
                                        .upcast()
                                }
                            ),
                        );
                    } else {
                        self.ports_pod_stack.set_visible_child_name("no-ports");
                    }
                } else {
                    self.ports_pod_stack.set_visible_child_name("pod");
                }
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerCard(ObjectSubclass<imp::ContainerCard>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for ContainerCard {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder()
            .property("container", container)
            .build()
    }
}

impl ContainerCard {
    pub(crate) fn activate(&self) {
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

                utils::navigation_view(self).push(&nav_page);
            }
        }
    }

    pub(crate) fn delete(&self) {
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Delete Container?"))
            .body_use_markup(true)
            .body(gettext(
                "All settings and all changes made within the container will be irreversibly lost",
            ))
            .build();

        dialog.add_responses(&[
            ("cancel", &gettext("_Cancel")),
            ("confirm", &gettext("_Confirm")),
        ]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);

        if glib::MainContext::default().block_on(dialog.choose_future(self)) == "confirm" {
            view::container::delete(self, self.container())
        }
    }

    fn bind_stats_fraction(&self, stats_expr: &gtk::Expression, progress_bar: &gtk::ProgressBar) {
        let percent_expr =
            stats_expr.chain_closure::<f64>(closure!(|_: Self, value: f64| value * 0.01));

        let target = adw::PropertyAnimationTarget::new(progress_bar, "fraction");
        let animation = adw::TimedAnimation::builder()
            .widget(progress_bar)
            .duration(750)
            .target(&target)
            .build();

        percent_expr.watch(
            Some(self),
            clone!(
                #[weak(rename_to = obj)]
                self,
                #[weak]
                progress_bar,
                #[strong]
                percent_expr,
                move || {
                    animation.set_value_from(progress_bar.fraction());
                    animation.set_value_to(percent_expr.evaluate_as(Some(&obj)).unwrap_or(0.0));
                    animation.play();
                }
            ),
        );

        let classes = utils::css_classes(progress_bar);

        #[rustfmt::skip]
        percent_expr.chain_closure::<Vec<String>>(closure!(|_: Self, value: f64| {
            classes
                .iter()
                .cloned()
                .chain(if value >= 0.8 {
                    Some(String::from(if value < 0.95 {
                        "warning"
                    } else {
                        "error"
                    }))
                } else {
                    None
                })
                .collect::<Vec<_>>()
        }))
        .bind(progress_bar, "css-classes", Some(self));
    }

    fn bind_stats_throughput(
        &self,
        stats_expr: &gtk::Expression,
        box_: &gtk::Box,
        label: &gtk::Label,
    ) {
        self.curr_value_expr(stats_expr)
            .chain_closure::<String>(closure!(|_: Self, value: u64| {
                gettext!(
                    // Translators: For example 5 MB / s.
                    "{} / s",
                    glib::format_size(value)
                )
            }))
            .bind(label, "label", Some(self));

        let css_classes = utils::css_classes(box_);
        self.curr_value_expr(stats_expr)
            .chain_closure::<Vec<String>>(closure!(|_: Self, value: u64| {
                css_classes
                    .iter()
                    .cloned()
                    .chain(if value > 0 {
                        None
                    } else {
                        Some("dim-label".to_string())
                    })
                    .collect::<Vec<_>>()
            }))
            .bind(box_, "css-classes", Some(self));
    }

    fn curr_value_expr(&self, stats_expr: &gtk::Expression) -> gtk::Expression {
        let prev_value = Cell::new(u64::MAX);

        stats_expr
            .chain_closure::<u64>(closure_local!(move |_: Self, value: u64| {
                let next_value = if prev_value.get() >= value {
                    0
                } else {
                    value - prev_value.get()
                };

                prev_value.set(value);
                next_value
            }))
            .upcast()
    }

    fn update_actions(&self) {
        if let Some(container) = self.container() {
            let imp = self.imp();

            imp.action_center_box.set_sensitive(
                !container.action_ongoing()
                    && !container.container_list().unwrap().is_selection_mode(),
            );

            let can_start_or_resume = container.can_start() || container.can_resume();
            let can_stop = container.can_stop();

            imp.start_or_resume_button
                .set_visible(!container.action_ongoing() && can_start_or_resume);
            imp.stop_button
                .set_visible(!container.action_ongoing() && can_stop);
            imp.spinning_button.set_visible(
                container.action_ongoing()
                    || (!imp.start_or_resume_button.is_visible() && !imp.stop_button.is_visible()),
            );

            self.action_set_enabled(ACTION_START_OR_RESUME, can_start_or_resume);
            self.action_set_enabled(ACTION_STOP, can_stop);
            self.action_set_enabled(ACTION_KILL, container.can_kill());
            self.action_set_enabled(ACTION_RESTART, container.can_restart());
            self.action_set_enabled(ACTION_PAUSE, container.can_pause());
            self.action_set_enabled(ACTION_DELETE, container.can_delete());
        }
    }
}
