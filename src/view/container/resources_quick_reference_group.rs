use std::cell::Cell;

use adw::subclass::prelude::PreferencesGroupImpl;
use adw::traits::AnimationExt;
use gettextrs::gettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::closure_local;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(
        resource = "/com/github/marhkb/Pods/ui/container/resources-quick-reference-group.ui"
    )]
    pub(crate) struct ResourcesQuickReferenceGroup {
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) cpu_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) cpu_progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) memory_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) memory_progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) network_down_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) network_up_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) block_down_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) block_up_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ResourcesQuickReferenceGroup {
        const NAME: &'static str = "PdsContainerResourcesQuickReferenceGroup";
        type Type = super::ResourcesQuickReferenceGroup;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ResourcesQuickReferenceGroup {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Container>("container")
                        .construct()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "container" => {
                    self.container.set(value.get().unwrap());
                }
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

            let stats_expr = Self::Type::this_expression("container")
                .chain_property::<model::Container>("stats");

            stats_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, stats: Option<model::BoxedContainerStats>| {
                        gettext!(
                            "{} %",
                            stats
                                .and_then(|stats| stats.cpu.map(|perc| format!("{perc:.1}")))
                                .unwrap_or_else(|| gettext("?")),
                        )
                    }
                ))
                .bind(&*self.cpu_label, "label", Some(obj));

            obj.bind_stats_fraction(
                stats_expr.upcast_ref(),
                |stats| stats.cpu,
                &self.cpu_progress_bar,
            );

            stats_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, stats: Option<model::BoxedContainerStats>| {
                        stats
                            .map(|stats| {
                                gettext!(
                                    "{} / {} ({} %)",
                                    stats
                                        .mem_usage
                                        .map(glib::format_size)
                                        .map(String::from)
                                        .unwrap_or_else(|| gettext("?")),
                                    stats
                                        .mem_limit
                                        .map(glib::format_size)
                                        .map(String::from)
                                        .unwrap_or_else(|| gettext("?")),
                                    stats
                                        .mem_perc
                                        .map(|perc| format!("{perc:.1}"))
                                        .unwrap_or_else(|| gettext("?")),
                                )
                            })
                            .unwrap_or_else(|| gettext("?"))
                    }
                ))
                .bind(&*self.memory_label, "label", Some(obj));

            obj.bind_stats_fraction(
                stats_expr.upcast_ref(),
                |stats| stats.mem_perc,
                &self.memory_progress_bar,
            );

            obj.bind_stats_throughput(
                stats_expr.upcast_ref(),
                |stats| stats.net_input,
                &self.network_down_label,
            );

            obj.bind_stats_throughput(
                stats_expr.upcast_ref(),
                |stats| stats.net_output,
                &self.network_up_label,
            );

            obj.bind_stats_throughput(
                stats_expr.upcast_ref(),
                |stats| stats.block_input,
                &self.block_down_label,
            );

            obj.bind_stats_throughput(
                stats_expr.upcast_ref(),
                |stats| stats.block_output,
                &self.block_up_label,
            );
        }
    }

    impl WidgetImpl for ResourcesQuickReferenceGroup {}
    impl PreferencesGroupImpl for ResourcesQuickReferenceGroup {}
}

glib::wrapper! {
    pub(crate) struct ResourcesQuickReferenceGroup(ObjectSubclass<imp::ResourcesQuickReferenceGroup>)
        @extends gtk::Widget, adw::PreferencesGroup,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ResourcesQuickReferenceGroup {
    fn bind_stats_fraction<F>(
        &self,
        stats_expr: &gtk::Expression,
        fraction_op: F,
        progress_bar: &gtk::ProgressBar,
    ) where
        F: Fn(model::BoxedContainerStats) -> Option<f64> + Clone + 'static,
    {
        let fraction_op_clone = fraction_op.clone();
        let percent_expr = stats_expr.chain_closure::<f64>(closure_local!(|_: Self,
                                                                           stats: Option<
            model::BoxedContainerStats,
        >| {
            stats
                .and_then(|stats| fraction_op_clone(stats).map(|perc| perc * 0.01))
                .unwrap_or_default()
        }));

        let target = adw::PropertyAnimationTarget::new(progress_bar, "fraction");
        let animation = adw::TimedAnimation::builder()
            .widget(progress_bar)
            .duration(750)
            .target(&target)
            .build();

        stats_expr.clone().watch(
            Some(self),
            clone!(@weak self as obj, @weak progress_bar => move || {
                animation.set_value_from(progress_bar.fraction());
                animation.set_value_to(percent_expr.evaluate_as(Some(&obj)).unwrap_or(0.0));
                animation.play();
            }),
        );

        let classes = utils::css_classes(progress_bar.upcast_ref());

        #[rustfmt::skip]
        stats_expr.chain_closure::<Vec<String>>(
            closure_local!(|_: Self, stats: Option<model::BoxedContainerStats>| {
                classes
                    .iter()
                    .cloned()
                    .chain(stats.and_then(|stats| {
                        fraction_op(stats).and_then(|perc| {
                            if perc >= 80. {
                                Some(String::from(if perc < 95. {
                                    "warning"
                                } else {
                                    "error"
                                }))
                            } else {
                                None
                            }
                        })
                    }))
                    .collect::<Vec<_>>()
            })
        )
        .bind(progress_bar, "css-classes", Some(self));
    }

    fn bind_stats_throughput<F>(
        &self,
        stats_expr: &gtk::Expression,
        throughput_op: F,
        label: &gtk::Label,
    ) where
        F: Fn(model::BoxedContainerStats) -> Option<u64> + 'static,
    {
        let prev_value = Cell::new(u64::MAX);

        stats_expr
            .chain_closure::<String>(closure_local!(move |_: Self,
                                                          stats: Option<
                model::BoxedContainerStats,
            >| {
                stats
                    .and_then(|stats| {
                        throughput_op(stats).map(|value| {
                            let s = gettext!(
                                // Translators: For example 5 MB / s.
                                "{} / s",
                                glib::format_size(if prev_value.get() >= value {
                                    0
                                } else {
                                    value - prev_value.get()
                                })
                            );

                            prev_value.set(value);

                            s
                        })
                    })
                    .unwrap_or_else(|| gettext("?"))
            }))
            .bind(label, "label", Some(self));
    }

    pub(crate) fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }
}
