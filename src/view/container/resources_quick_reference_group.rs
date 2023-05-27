use std::cell::Cell;

use adw::subclass::prelude::PreferencesGroupImpl;
use adw::traits::AnimationExt;
use gettextrs::gettext;
use gettextrs::ngettext;
use glib::clone;
use glib::closure;
use glib::closure_local;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ResourcesQuickReferenceGroup)]
    #[template(
        resource = "/com/github/marhkb/Pods/ui/container/resources-quick-reference-group.ui"
    )]
    pub(crate) struct ResourcesQuickReferenceGroup {
        #[property(get, set, construct, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) cpu_name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) cpu_percent_label: TemplateChild<gtk::Label>,
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
            let cpus_expr = container_expr
                .chain_property::<model::Container>("container-list")
                .chain_property::<model::ContainerList>("client")
                .chain_property::<model::Client>("cpus");
            let stats_expr = container_expr.chain_property::<model::Container>("stats");

            cpus_expr
                .chain_closure::<String>(closure!(|_: Self::Type, cpus: i64| {
                    if cpus > 0 {
                        ngettext!(
                            "Processor ({} CPU)",
                            "Processor ({} CPUs)",
                            cpus as u32,
                            cpus
                        )
                    } else {
                        gettext("Processor (? CPUs)")
                    }
                }))
                .bind(&*self.cpu_name_label, "label", Some(obj));

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
                .bind(&*self.cpu_percent_label, "label", Some(obj));

            #[rustfmt::skip]
            obj.bind_stats_fraction(
                gtk::ClosureExpression::new::<f64>(
                    [cpus_expr.upcast_ref(), stats_expr.upcast_ref()],
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
                &self.block_down_label,
            );

            obj.bind_stats_throughput(
                stats_expr
                    .chain_closure::<u64>(closure!(
                        |_: Self::Type, stats: Option<model::BoxedContainerStats>| {
                            stats.and_then(|stats| stats.block_output).unwrap_or(0)
                        }
                    ))
                    .upcast_ref(),
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

        let classes = utils::css_classes(progress_bar.upcast_ref());

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

    fn bind_stats_throughput(&self, stats_expr: &gtk::Expression, label: &gtk::Label) {
        let prev_value = Cell::new(u64::MAX);

        stats_expr
            .chain_closure::<String>(closure_local!(move |_: Self, value: u64| {
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
            }))
            .bind(label, "label", Some(self));
    }
}
