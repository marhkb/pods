use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_START_OR_RESUME: &str = "pod-details-page.start";
const ACTION_STOP: &str = "pod-details-page.stop";
const ACTION_KILL: &str = "pod-details-page.kill";
const ACTION_RESTART: &str = "pod-details-page.restart";
const ACTION_PAUSE: &str = "pod-details-page.pause";
const ACTION_RESUME: &str = "pod-details-page.resume";
const ACTION_DELETE: &str = "pod-details-page.delete";
const ACTION_INSPECT_POD: &str = "pod-details-page.inspect-pod";
const ACTION_GENERATE_KUBE: &str = "pod-details-page.generate-kube";
const ACTION_SHOW_PROCESSES: &str = "pod-details-page.show-processes";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PodDetailsPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/pod_details_page.ui")]
    pub(crate) struct PodDetailsPage {
        pub(super) handler_id: RefCell<Option<glib::SignalHandlerId>>,
        #[property(get, set = Self::set_pod, construct, nullable)]
        pub(super) pod: glib::WeakRef<model::Pod>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) action_row: TemplateChild<adw::PreferencesRow>,
        #[template_child]
        pub(super) start_or_resume_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) stop_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) spinning_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) id_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) created_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) hostname_row: TemplateChild<adw::ActionRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodDetailsPage {
        const NAME: &'static str = "PdsPodDetailsPage";
        type Type = super::PodDetailsPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_START_OR_RESUME, None, |widget, _, _| {
                if widget
                    .pod()
                    .map(|pod| pod.status().can_start())
                    .unwrap_or(false)
                {
                    view::pod::start(widget, widget.pod());
                } else {
                    view::pod::resume(widget, widget.pod());
                }
            });
            klass.install_action(ACTION_STOP, None, |widget, _, _| {
                view::pod::stop(widget, widget.pod());
            });
            klass.install_action(ACTION_KILL, None, |widget, _, _| {
                view::pod::kill(widget, widget.pod());
            });
            klass.install_action(ACTION_RESTART, None, |widget, _, _| {
                view::pod::restart(widget, widget.pod());
            });
            klass.install_action(ACTION_PAUSE, None, |widget, _, _| {
                view::pod::pause(widget, widget.pod());
            });
            klass.install_action(ACTION_RESUME, None, |widget, _, _| {
                view::pod::resume(widget, widget.pod());
            });
            klass.install_action(ACTION_DELETE, None, |widget, _, _| {
                view::pod::delete(widget, widget.pod());
            });

            klass.install_action(ACTION_INSPECT_POD, None, |widget, _, _| {
                widget.show_inspection();
            });
            klass.install_action(ACTION_GENERATE_KUBE, None, |widget, _, _| {
                widget.show_kube();
            });
            klass.install_action(ACTION_SHOW_PROCESSES, None, |widget, _, _| {
                widget.show_processes();
            });

            // For displaying a mnemonic.
            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                view::ContainersGroup::action_create_container(),
            );
            klass.install_action(
                view::ContainersGroup::action_create_container(),
                None,
                move |widget, _, _| {
                    widget.create_container();
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodDetailsPage {
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

            let pod_expr = Self::Type::this_expression("pod");
            let details_expr = pod_expr.chain_property::<model::Pod>("details");
            let status_expr = pod_expr.chain_property::<model::Pod>("status");
            let hostname_expr = details_expr.chain_property::<model::PodDetails>("hostname");

            details_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, details: Option<model::PodDetails>| details
                        .map(|_| "loaded")
                        .unwrap_or("loading")
                ))
                .bind(&*self.stack, "visible-child-name", Some(obj));

            pod_expr
                .chain_property::<model::Pod>("id")
                .chain_closure::<String>(closure!(
                    |_: Self::Type, id: &str| utils::format_id(id).to_owned()
                ))
                .bind(&*self.id_row, "subtitle", Some(obj));

            gtk::ClosureExpression::new::<String>(
                &[
                    Self::Type::this_expression("root")
                        .chain_property::<gtk::Window>("application")
                        .chain_property::<crate::Application>("ticks"),
                    pod_expr.chain_property::<model::Pod>("created"),
                ],
                closure!(|_: Self::Type, _ticks: u64, created: i64| {
                    utils::format_ago(utils::timespan_now(created))
                }),
            )
            .bind(&*self.created_row, "subtitle", Some(obj));

            status_expr
                .chain_closure::<String>(closure!(|_: Self::Type, status: model::PodStatus| {
                    status.to_string()
                }))
                .bind(&*self.status_label, "label", Some(obj));

            let css_classes = utils::css_classes(&*self.status_label);
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::PodStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(String::from(super::super::pod_status_css_class(
                                status,
                            ))))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.status_label, "css-classes", Some(obj));

            hostname_expr.bind(&*self.hostname_row, "subtitle", Some(obj));
            hostname_expr
                .chain_closure::<bool>(closure!(
                    |_: Self::Type, hostname: String| !hostname.is_empty()
                ))
                .bind(&*self.hostname_row, "visible", Some(obj));

            status_expr.watch(
                Some(obj),
                clone!(
                    #[weak]
                    obj,
                    move || obj.update_actions()
                ),
            );
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for PodDetailsPage {}

    impl PodDetailsPage {
        pub(super) fn set_pod(&self, value: Option<&model::Pod>) {
            let obj = &*self.obj();
            if obj.pod().as_ref() == value {
                return;
            }

            if let Some(pod) = obj.pod() {
                pod.disconnect(self.handler_id.take().unwrap());
            }

            if let Some(pod) = value {
                if pod.details().is_none() {
                    pod.inspect_and_update(clone!(
                        #[weak]
                        obj,
                        move |e| {
                            utils::show_error_toast(
                                &obj,
                                &gettext("Error on loading pod details"),
                                &e.to_string(),
                            );
                        }
                    ));
                }

                let handler_id = pod.connect_deleted(clone!(
                    #[weak]
                    obj,
                    move |pod| {
                        utils::show_toast(&obj, gettext!("Pod '{}' has been deleted", pod.name()));
                        utils::navigation_view(&obj).pop();
                    }
                ));
                self.handler_id.replace(Some(handler_id));
            }

            self.pod.set(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct PodDetailsPage(ObjectSubclass<imp::PodDetailsPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Pod> for PodDetailsPage {
    fn from(pod: &model::Pod) -> Self {
        glib::Object::builder().property("pod", pod).build()
    }
}

impl PodDetailsPage {
    fn update_actions(&self) {
        let Some(pod) = self.pod() else {
            return;
        };

        let imp = self.imp();

        imp.action_row.set_sensitive(!pod.status().is_transition());

        let can_start_or_resume = pod.status().can_start() || pod.status().can_resume();
        let can_stop = pod.status().can_stop();

        imp.start_or_resume_button.set_visible(can_start_or_resume);
        imp.stop_button.set_visible(can_stop);
        imp.spinning_button
            .set_visible(!imp.start_or_resume_button.is_visible() && !imp.stop_button.is_visible());

        self.action_set_enabled(ACTION_START_OR_RESUME, can_start_or_resume);
        self.action_set_enabled(ACTION_STOP, can_stop);
        self.action_set_enabled(ACTION_KILL, pod.status().can_kill());
        self.action_set_enabled(ACTION_RESTART, pod.status().can_restart());
        self.action_set_enabled(ACTION_PAUSE, pod.status().can_pause());
        self.action_set_enabled(ACTION_DELETE, pod.status().can_delete());
    }

    fn show_inspection(&self) {
        self.show_kube_inspection_or_kube(view::ScalableTextViewMode::Inspect);
    }

    fn show_kube(&self) {
        self.show_kube_inspection_or_kube(view::ScalableTextViewMode::Kube);
    }

    fn show_kube_inspection_or_kube(&self, mode: view::ScalableTextViewMode) {
        self.exec_action(|| {
            if let Some(pod) = self.pod() {
                let weak_ref = glib::WeakRef::new();
                weak_ref.set(Some(&pod));

                utils::navigation_view(self).push(
                    &adw::NavigationPage::builder()
                        .child(&view::ScalableTextViewPage::from(view::Entity::Pod {
                            pod: weak_ref,
                            mode,
                        }))
                        .build(),
                );
            }
        });
    }

    fn show_processes(&self) {
        self.exec_action(|| {
            if let Some(pod) = self.pod() {
                utils::navigation_view(self).push(
                    &adw::NavigationPage::builder()
                        .child(&view::TopPage::from(&pod))
                        .build(),
                );
            }
        });
    }

    fn create_container(&self) {
        self.exec_action(|| {
            view::pod::create_container(self, self.pod());
        });
    }

    fn exec_action<F: Fn()>(&self, op: F) {
        if utils::navigation_view(self)
            .visible_page()
            .filter(|page| page.child().as_ref() == Some(self.upcast_ref()))
            .is_some()
        {
            op();
        }
    }
}
