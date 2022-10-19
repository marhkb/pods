use std::cell::RefCell;

use adw::subclass::prelude::ActionRowImpl;
use adw::subclass::prelude::PreferencesRowImpl;
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
        pub(super) status_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) stats_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) cpu_bar: TemplateChild<view::CircularProgressBar>,
        #[template_child]
        pub(super) mem_bar: TemplateChild<view::CircularProgressBar>,
        #[template_child]
        pub(super) health_status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) end_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsContainerRow";
        type Type = super::Row;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("container-row.activate", None, move |widget, _, _| {
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
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "container" => self.instance().set_container(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container" => self.instance().container().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            let container_expr = Self::Type::this_expression("container");

            let selection_mode_expr = container_expr
                .chain_property::<model::Container>("container-list")
                .chain_property::<model::ContainerList>("selection-mode");

            selection_mode_expr.bind(&*self.check_button, "visible", Some(obj));
            selection_mode_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, is_selection_mode: bool| {
                    !is_selection_mode
                }))
                .bind(&*self.end_box, "visible", Some(obj));

            let stats_expr = container_expr.chain_property::<model::Container>("stats");
            let health_status_expr =
                container_expr.chain_property::<model::Container>("health-status");
            let status_expr = container_expr.chain_property::<model::Container>("status");

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
                .bind(&*self.status_image, "icon-name", Some(obj));

            let css_classes = self.status_image.css_classes();
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
                .bind(&*self.status_image, "css-classes", Some(obj));

            container_expr
                .chain_property::<model::Container>("name")
                .chain_closure::<String>(closure!(|_: glib::Object, name: Option<String>| {
                    utils::escape(&utils::format_option(name))
                }))
                .bind(obj, "title", Some(obj));

            container_expr
                .chain_property::<model::Container>("image-name")
                .chain_closure::<String>(closure!(|_: glib::Object, name: Option<String>| {
                    utils::escape(&utils::format_option(name))
                }))
                .bind(obj, "subtitle", Some(obj));

            status_expr
                .chain_closure::<bool>(closure!(
                    |_: glib::Object, status: model::ContainerStatus| matches!(
                        status,
                        model::ContainerStatus::Running
                    )
                ))
                .bind(&*self.stats_box, "visible", Some(obj));

            obj.bind_stats_percentage(stats_expr.upcast_ref(), |stats| stats.cpu, &*self.cpu_bar);
            obj.bind_stats_percentage(
                stats_expr.upcast_ref(),
                |stats| stats.mem_perc,
                &*self.mem_bar,
            );

            health_status_expr
                .chain_closure::<String>(closure!(
                    |_: glib::Object, status: model::ContainerHealthStatus| status.to_string()
                ))
                .bind(&*self.health_status_label, "label", Some(obj));

            gtk::ClosureExpression::new::<bool>(
                &[status_expr.upcast_ref(), health_status_expr.upcast_ref()],
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
                    |_: glib::Object, status: model::ContainerHealthStatus| {
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
        }
    }

    impl WidgetImpl for Row {}
    impl ListBoxRowImpl for Row {}
    impl PreferencesRowImpl for Row {}
    impl ActionRowImpl for Row {}
}

glib::wrapper! {
    pub(crate) struct Row(ObjectSubclass<imp::Row>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for Row {
    fn from(container: &model::Container) -> Self {
        glib::Object::new::<Self>(&[("container", container)])
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
        let perc_expr = stats_expr.chain_closure::<f64>(closure_local!(|_: glib::Object,
                                                                        stats: Option<
            model::BoxedContainerStats,
        >| {
            stats
                .and_then(|stats| fraction_op(stats).map(|perc| perc as f64 * 0.01))
                .unwrap_or_default()
        }));

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
                utils::find_leaflet_overlay(self)
                    .show_details(&view::ContainerDetailsPage::from(container));
            }
        }
    }
}
