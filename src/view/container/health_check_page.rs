use gettextrs::gettext;
use gettextrs::ngettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_RUN_HEALTH_COMMAND: &str = "container-health-check-page.run-health-check";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::HealthCheckPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container/health-check-page.ui")]
    pub(crate) struct HealthCheckPage {
        #[property(get, set = Self::set_container, construct, explicit_notify, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) command_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) interval_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) retries_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) timeout_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) failing_streak_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) log_list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HealthCheckPage {
        const NAME: &'static str = "PdsContainerHealthCheckPage";
        type Type = super::HealthCheckPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_RUN_HEALTH_COMMAND, None, |widget, _, _| {
                widget.run_health_check()
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for HealthCheckPage {
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

            let health_status_expr =
                container_expr.chain_property::<model::Container>("health_status");
            let data_expr = container_expr.chain_property::<model::Container>("data");

            gtk::ClosureExpression::new::<bool>(
                [&container_expr.chain_property::<model::Container>("status"), &health_status_expr],
                closure!(|_: Self::Type,
                          _: model::ContainerStatus,
                          _: model::ContainerHealthStatus| false),
            )
            .watch(Some(obj), clone!(@weak obj => move || {
                obj.action_set_enabled(
                    ACTION_RUN_HEALTH_COMMAND,
                    obj.container()
                        .map(|container| {
                            container.health_status() != model::ContainerHealthStatus::Unconfigured
                                && container.status() == model::ContainerStatus::Running
                        })
                        .unwrap_or(false),
                );
            }));

            health_status_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, status: model::ContainerHealthStatus| status.to_string()
                ))
                .bind(&*self.status_label, "label", Some(obj));

            let css_classes = utils::css_classes(self.status_label.upcast_ref());
            health_status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::ContainerHealthStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(String::from(
                                super::super::container_health_status_css_class(status),
                            )))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.status_label, "css-classes", Some(obj));

            data_expr.watch(Some(obj), clone!(@weak obj => move || {
                let model = obj
                    .container()
                    .as_ref()
                    .and_then(model::Container::data)
                    .as_ref()
                    .map(model::ContainerData::health_check_log_list);

                if let Some(ref model) = model {
                    obj.set_list_box_visibility(model.upcast_ref());
                    model.connect_items_changed(clone!(@weak obj => move |model, _, _, _| {
                        obj.set_list_box_visibility(model.upcast_ref());
                    }));
                }

                let sort_model = gtk::SortListModel::new(model, Some(gtk::CustomSorter::new(|item1, item2| {
                    let log1 = item1.downcast_ref::<model::HealthCheckLog>().unwrap();
                    let log2 = item2.downcast_ref::<model::HealthCheckLog>().unwrap();
                    log2.start().cmp(&log1.start()).into()
                })));

                obj.imp().log_list_box.bind_model(Some(&sort_model), move |log| {
                    view::HealthCheckLogRow::from(log.downcast_ref().unwrap()).upcast()
                })
            }));
        }

        fn dispose(&self) {
            utils::ChildIter::from(self.obj().upcast_ref()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for HealthCheckPage {}

    impl HealthCheckPage {
        pub(crate) fn set_container(&self, value: Option<&model::Container>) {
            let obj = &*self.obj();
            if obj.container().as_ref() == value {
                return;
            }

            if let Some(config) = value
                .and_then(model::Container::data)
                .as_ref()
                .and_then(model::ContainerData::health_config)
            {
                self.command_row.set_value(
                    &config
                        .test
                        .as_ref()
                        .map(|s| s.join(" "))
                        .unwrap_or_default(),
                );
                self.interval_row.set_value(
                    &config
                        .interval
                        .map(|nanos| {
                            let secs = nanos / 1000000000;
                            ngettext!("{} second", "{} seconds", secs as u32, secs)
                        })
                        .unwrap_or_default(),
                );
                self.retries_row.set_value(
                    &config
                        .retries
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                );
                self.timeout_row.set_value(
                    &config
                        .timeout
                        .map(|nanos| {
                            let secs = nanos / 1000000000;
                            ngettext!("{} second", "{} seconds", secs as u32, secs)
                        })
                        .unwrap_or_default(),
                );
            }

            self.container.set(value);
            obj.notify("container");
        }
    }
}

glib::wrapper! {
    pub(crate) struct HealthCheckPage(ObjectSubclass<imp::HealthCheckPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for HealthCheckPage {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder()
            .property("container", container)
            .build()
    }
}

impl HealthCheckPage {
    fn set_list_box_visibility(&self, model: &gio::ListModel) {
        self.imp().log_list_box.set_visible(model.n_items() > 0);
    }

    pub(crate) fn run_health_check(&self) {
        if let Some(container) = self.container().as_ref().and_then(model::Container::api) {
            utils::do_async(
                async move { container.healthcheck().await },
                clone!(@weak self as obj => move |result| if let Err(e) = result {
                    utils::show_error_toast(
                        obj.upcast_ref(),
                        &gettext("Error on running health check"),
                        &e.to_string()
                    );
                }),
            );
        }
    }
}
