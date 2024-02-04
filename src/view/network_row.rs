use std::cell::Cell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use glib::closure;
use glib::closure_local;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;

use crate::model;
use crate::utils;

const ACTION_RENAME: &str = "network-card.rename";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::NetworkRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/network_row.ui")]
    pub(crate) struct NetworkRow {
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[property(get, set, construct, nullable)]
        pub(super) network: glib::WeakRef<model::Network>,
        #[template_child]
        pub(super) name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) id_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) edit_select_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) selection_check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) driver_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) public_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) public_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) dns_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) dns_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) ipv6_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) ipv6_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) subnets_list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NetworkRow {
        const NAME: &'static str = "PdsNetworkRow";
        type Type = super::NetworkRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.set_css_name("networkcard");

            klass.install_action("network-card.activate", None, |widget, _, _| {
                widget.activate();
            });

            klass.install_action(ACTION_RENAME, None, |widget, _, _| {
                widget.rename();
            });

            // klass.install_action(ACTION_START_OR_RESUME, None, |widget, _, _| {
            //     if widget.container().map(|c| c.can_start()).unwrap_or(false) {
            //         view::container::start(widget.upcast_ref());
            //     } else {
            //         view::container::resume(widget.upcast_ref());
            //     }
            // });
            // klass.install_action(ACTION_STOP, None, |widget, _, _| {
            //     view::container::stop(widget, widget.network());
            // });
            // klass.install_action(ACTION_KILL, None, |widget, _, _| {
            //     view::container::kill(widget, widget.network());
            // });
            // klass.install_action(ACTION_RESTART, None, |widget, _, _| {
            //     view::container::restart(widget, widget.network());
            // });
            // klass.install_action(ACTION_PAUSE, None, |widget, _, _| {
            //     view::container::pause(widget, widget.network());
            // });
            // klass.install_action(ACTION_RESUME, None, |widget, _, _| {
            //     view::container::resume(widget, widget.network());
            // });
            // klass.install_action(ACTION_DELETE, None, |widget, _, _| {
            //     widget.delete();
            // });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NetworkRow {
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

            let network_expr = Self::Type::this_expression("network");
            let network_inner_expr = network_expr.chain_property::<model::Network>("inner");
            let network_name_expr = network_inner_expr.chain_closure::<String>(closure!(
                |_: Self::Type, inner: model::BoxedNetwork| { inner.name.clone() }
            ));
            let network_id_expr = network_inner_expr.chain_closure::<String>(closure!(
                |_: Self::Type, inner: model::BoxedNetwork| { inner.id.clone() }
            ));
            let network_short_id_expr =
                network_id_expr.chain_closure::<String>(closure!(|_: Self::Type, id: &str| {
                    utils::format_id(id)
                }));
            let driver_name_expr = network_inner_expr.chain_closure::<String>(closure!(
                |_: Self::Type, inner: model::BoxedNetwork| { inner.driver.clone() }
            ));

            let is_public_expr = network_inner_expr.chain_closure::<bool>(closure!(
                |_: Self::Type, inner: model::BoxedNetwork| {
                    !inner.internal.clone().unwrap_or(true)
                }
            ));
            let public_icon_expr =
                is_public_expr.chain_closure::<String>(closure!(|_: Self::Type, enabled: bool| {
                    if enabled {
                        "check-round-outline2-symbolic"
                    } else {
                        "minus-circle-outline-symbolic"
                    }
                }));
            let css_classes = utils::css_classes(&self.public_box.get());
            let public_css_expr = is_public_expr.chain_closure::<Vec<String>>(closure!(
                |_: Self::Type, enabled: bool| {
                    css_classes
                        .iter()
                        .cloned()
                        .chain(Some(String::from(if enabled {
                            "network-public"
                        } else {
                            "network-public-no"
                        })))
                        .collect::<Vec<_>>()
                }
            ));

            let is_dns_expr = network_inner_expr.chain_closure::<bool>(closure!(
                |_: Self::Type, inner: model::BoxedNetwork| {
                    !inner.dns_enabled.clone().unwrap_or(true)
                }
            ));
            let dns_icon_expr =
                is_dns_expr.chain_closure::<String>(closure!(|_: Self::Type, enabled: bool| {
                    if enabled {
                        "check-round-outline2-symbolic"
                    } else {
                        "minus-circle-outline-symbolic"
                    }
                }));
            let css_classes = utils::css_classes(&self.dns_box.get());
            let dns_css_expr = is_public_expr.chain_closure::<Vec<String>>(closure!(
                |_: Self::Type, enabled: bool| {
                    css_classes
                        .iter()
                        .cloned()
                        .chain(Some(String::from(if enabled {
                            "network-dns"
                        } else {
                            "network-dns-no"
                        })))
                        .collect::<Vec<_>>()
                }
            ));

            let is_ipv6_expr = network_inner_expr.chain_closure::<bool>(closure!(
                |_: Self::Type, inner: model::BoxedNetwork| {
                    inner.ipv_6_enabled.clone().unwrap_or_default()
                }
            ));
            let ipv6_icon_expr =
                is_ipv6_expr.chain_closure::<String>(closure!(|_: Self::Type, enabled: bool| {
                    if enabled {
                        "check-round-outline2-symbolic"
                    } else {
                        "minus-circle-outline-symbolic"
                    }
                }));
            let css_classes = utils::css_classes(&self.ipv6_box.get());
            let ipv6_css_expr = is_ipv6_expr.chain_closure::<Vec<String>>(closure!(
                |_: Self::Type, enabled: bool| {
                    css_classes
                        .iter()
                        .cloned()
                        .chain(Some(String::from(if enabled {
                            "network-ipv6"
                        } else {
                            "network-ipv6-no"
                        })))
                        .collect::<Vec<_>>()
                }
            ));

            let network_to_be_deleted_expr =
                network_expr.chain_property::<model::Network>("to-be-deleted");

            let network_list_expr = network_expr.chain_property::<model::Network>("network-list");

            let selection_mode_expr =
                network_list_expr.chain_property::<model::NetworkList>("selection-mode");

            selection_mode_expr
                .chain_closure::<String>(closure!(|_: Self::Type, is_selection_mode: bool| {
                    if is_selection_mode { "select" } else { "edit" }
                }))
                .bind(
                    &self.edit_select_stack.get(),
                    "visible-child-name",
                    Some(obj),
                );

            gtk::ClosureExpression::new::<String>(
                [
                    network_name_expr.upcast_ref(),
                    &network_to_be_deleted_expr.upcast_ref(),
                ],
                closure!(|_: Self::Type, name: String, to_be_deleted: bool| {
                    if to_be_deleted {
                        format!("<s>{name}</s>")
                    } else {
                        name
                    }
                }),
            )
            .bind(&*self.name_label, "label", Some(obj));

            network_short_id_expr.bind(&self.id_label.get(), "label", Some(obj));

            driver_name_expr.bind(&self.driver_label.get(), "label", Some(obj));

            public_icon_expr.bind(&self.public_icon.get(), "icon-name", Some(obj));
            public_css_expr.bind(&self.public_box.get(), "css-classes", Some(obj));
            dns_icon_expr.bind(&self.dns_icon.get(), "icon-name", Some(obj));
            dns_css_expr.bind(&self.dns_box.get(), "css-classes", Some(obj));
            ipv6_icon_expr.bind(&self.ipv6_icon.get(), "icon-name", Some(obj));
            ipv6_css_expr.bind(&self.ipv6_box.get(), "css-classes", Some(obj));

            network_expr
                .chain_property::<model::Network>("action-ongoing")
                .watch(Some(obj), clone!(@weak obj => move || obj.update_actions()));
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for NetworkRow {}

    #[gtk::template_callbacks]
    impl NetworkRow {
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
            println!("{key}");
            match key {
                gdk::Key::Return => {
                    self.obj().activate();
                    glib::ControlFlow::Continue
                }
                _ => glib::ControlFlow::Break,
            }
        }

        #[template_callback]
        fn on_notify_network(&self) {
            let mut bindings = self.bindings.borrow_mut();
            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }

            let obj = &*self.obj();

            if let Some(network) = obj.network() {
                let binding = network
                    .bind_property("selected", &*self.selection_check_button, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();

                bindings.push(binding);

                network
                    .inner()
                    .subnets
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .for_each(|subnet| {
                        let box_ = gtk::Box::builder()
                            .spacing(9)
                            .halign(gtk::Align::Center)
                            .homogeneous(true)
                            .css_classes(["caption"])
                            .build();

                        box_.append(&gtk::Label::new(subnet.gateway.as_deref()));
                        box_.append(&gtk::Label::new(subnet.subnet.as_deref()));

                        let row = gtk::ListBoxRow::builder()
                            .activatable(false)
                            .selectable(false)
                            .focusable(false)
                            .child(&box_)
                            .build();

                        self.subnets_list_box.append(&row);
                    });
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct NetworkRow(ObjectSubclass<imp::NetworkRow>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Network> for NetworkRow {
    fn from(network: &model::Network) -> Self {
        glib::Object::builder().property("network", network).build()
    }
}

impl NetworkRow {
    pub(crate) fn activate(&self) {
        if let Some(network) = self.network().as_ref() {
            // if network
            //     .container_list()
            //     .map(|list| list.is_selection_mode())
            //     .unwrap_or(false)
            // {
            //     network.select();
            // } else {
            //     let nav_page = adw::NavigationPage::builder()
            //         .child(&view::ContainerDetailsPage::from(network))
            //         .build();

            //     Self::this_expression("container")
            //         .chain_property::<model::Container>("name")
            //         .chain_closure::<String>(closure!(|_: Self, name: &str| gettext!(
            //             "Container {}",
            //             name
            //         )))
            //         .bind(&nav_page, "title", Some(self));

            //     utils::navigation_view(self.upcast_ref()).push(&nav_page);
            // }
        }
    }

    pub(crate) fn rename(&self) {
        // if let Some(container) = self.network() {
        //     let dialog = view::ContainerRenameDialog::from(&container);
        //     dialog.set_transient_for(Some(&utils::root(self.upcast_ref())));
        //     dialog.present();
        // }
    }

    pub(crate) fn delete(&self) {
        let dialog = adw::MessageDialog::builder()
            .heading(gettext("Delete Container?"))
            .body_use_markup(true)
            .body(gettext(
                "All settings and all changes made within the container will be irreversibly lost",
            ))
            .transient_for(&utils::root(self))
            .build();

        dialog.add_responses(&[
            ("cancel", &gettext("_Cancel")),
            ("confirm", &gettext("_Confirm")),
        ]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);

        if glib::MainContext::default().block_on(dialog.choose_future()) == "confirm" {
            // view::container::delete(&self)
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
            clone!(@weak self as obj, @weak progress_bar, @strong percent_expr => move || {
                animation.set_value_from(progress_bar.fraction());
                animation.set_value_to(percent_expr.evaluate_as(Some(&obj)).unwrap_or(0.0));
                animation.play();
            }),
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
        if let Some(container) = self.network() {
            let imp = self.imp();

            // imp.action_center_box.set_sensitive(
            //     !container.action_ongoing()
            //         && !container.container_list().unwrap().is_selection_mode(),
            // );

            // let can_start_or_resume = container.can_start() || container.can_resume();
            // let can_stop = container.can_stop();

            // imp.start_or_resume_button
            //     .set_visible(!container.action_ongoing() && can_start_or_resume);
            // imp.stop_button
            //     .set_visible(!container.action_ongoing() && can_stop);
            // imp.spinning_button.set_visible(
            //     container.action_ongoing()
            //         || (!imp.start_or_resume_button.is_visible() && !imp.stop_button.is_visible()),
            // );

            // self.action_set_enabled(ACTION_START_OR_RESUME, can_start_or_resume);
            // self.action_set_enabled(ACTION_STOP, can_stop);
            // self.action_set_enabled(ACTION_KILL, container.can_kill());
            // self.action_set_enabled(ACTION_RESTART, container.can_restart());
            // self.action_set_enabled(ACTION_PAUSE, container.can_pause());
            // self.action_set_enabled(ACTION_DELETE, container.can_delete());
        }
    }
}
