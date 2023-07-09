use std::cell::RefCell;

use adw::traits::NavigationPageExt;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_RENAME: &str = "container-details-page.rename";
const ACTION_COMMIT: &str = "container-details-page.commit";
const ACTION_GET_FILES: &str = "container-details-page.get-files";
const ACTION_PUT_FILES: &str = "container-details-page.put-files";
const ACTION_SHOW_HEALTH_DETAILS: &str = "container-details-page.show-health-details";
const ACTION_SHOW_IMAGE_DETAILS: &str = "container-details-page.show-image-details";
const ACTION_SHOW_POD_DETAILS: &str = "container-details-page.show-pod-details";
const ACTION_START_OR_RESUME: &str = "container-details-page.start";
const ACTION_STOP: &str = "container-details-page.stop";
const ACTION_KILL: &str = "container-details-page.kill";
const ACTION_RESTART: &str = "container-details-page.restart";
const ACTION_PAUSE: &str = "container-details-page.pause";
const ACTION_RESUME: &str = "container-details-page.resume";
const ACTION_DELETE: &str = "container-details-page.delete";

const ACTION_INSPECT: &str = "container-details-page.inspect";
const ACTION_GENERATE_KUBE: &str = "container-details-page.generate-kube";
const ACTION_SHOW_TTY: &str = "container-details-page.show-tty";
const ACTION_SHOW_LOG: &str = "container-details-page.show-log";
const ACTION_SHOW_PROCESSES: &str = "container-details-page.show-processes";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerDetailsPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_details_page.ui")]
    pub(crate) struct ContainerDetailsPage {
        pub(super) handler_id: RefCell<Option<glib::SignalHandlerId>>,
        #[property(get, set = Self::set_container, construct, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) action_row: TemplateChild<adw::PreferencesRow>,
        #[template_child]
        pub(super) start_or_resume_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) stop_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) spinning_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) volumes_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) volumes_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) resources: TemplateChild<view::ContainerResources>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerDetailsPage {
        const NAME: &'static str = "PdsContainerDetailsPage";
        type Type = super::ContainerDetailsPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_RENAME, None, |widget, _, _| {
                widget.rename();
            });
            klass.install_action(ACTION_COMMIT, None, |widget, _, _| {
                widget.commit();
            });
            klass.install_action(ACTION_GET_FILES, None, |widget, _, _| {
                widget.get_files();
            });
            klass.install_action(ACTION_PUT_FILES, None, |widget, _, _| {
                widget.put_files();
            });
            klass.install_action(ACTION_SHOW_HEALTH_DETAILS, None, |widget, _, _| {
                widget.show_health_details();
            });
            klass.install_action(ACTION_SHOW_IMAGE_DETAILS, None, |widget, _, _| {
                widget.show_image_details();
            });
            klass.install_action(ACTION_SHOW_POD_DETAILS, None, |widget, _, _| {
                widget.show_pod_details();
            });
            klass.install_action(ACTION_START_OR_RESUME, None, |widget, _, _| {
                if widget.container().map(|c| c.can_start()).unwrap_or(false) {
                    view::container::start(widget.upcast_ref());
                } else {
                    view::container::resume(widget.upcast_ref());
                }
            });
            klass.install_action(ACTION_STOP, None, |widget, _, _| {
                view::container::stop(widget.upcast_ref());
            });
            klass.install_action(ACTION_KILL, None, |widget, _, _| {
                view::container::kill(widget.upcast_ref());
            });
            klass.install_action(ACTION_RESTART, None, |widget, _, _| {
                view::container::restart(widget.upcast_ref());
            });
            klass.install_action(ACTION_PAUSE, None, |widget, _, _| {
                view::container::pause(widget.upcast_ref());
            });
            klass.install_action(ACTION_RESUME, None, |widget, _, _| {
                view::container::resume(widget.upcast_ref());
            });
            klass.install_action(ACTION_DELETE, None, |widget, _, _| {
                view::container::delete(widget.upcast_ref());
            });
            klass.install_action(ACTION_INSPECT, None, |widget, _, _| {
                widget.show_inspection();
            });
            klass.install_action(ACTION_GENERATE_KUBE, None, |widget, _, _| {
                widget.show_kube();
            });
            klass.install_action(ACTION_SHOW_TTY, None, |widget, _, _| {
                widget.show_tty();
            });
            klass.install_action(ACTION_SHOW_LOG, None, |widget, _, _| {
                widget.show_log();
            });
            klass.install_action(ACTION_SHOW_PROCESSES, None, |widget, _, _| {
                widget.show_processes();
            });

            klass.add_binding_action(
                gdk::Key::F2,
                gdk::ModifierType::empty(),
                ACTION_RENAME,
                None,
            );
            klass.add_binding_action(
                gdk::Key::K,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_COMMIT,
                None,
            );
            klass.add_binding_action(
                gdk::Key::D,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_GET_FILES,
                None,
            );
            klass.add_binding_action(
                gdk::Key::U,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_PUT_FILES,
                None,
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerDetailsPage {
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

            status_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, status: model::ContainerStatus| {
                    status == model::ContainerStatus::Running
                }))
                .bind(&*self.resources, "visible", Some(obj));

            status_expr.watch(Some(obj), clone!(@weak obj => move || obj.update_actions()));
            container_expr
                .chain_property::<model::Container>("action-ongoing")
                .watch(Some(obj), clone!(@weak obj => move || obj.update_actions()));

            container_expr
                .chain_property::<model::Container>("health_status")
                .watch(
                    Some(obj),
                    clone!(@weak obj => move || {
                        obj.action_set_enabled(
                            ACTION_SHOW_HEALTH_DETAILS,
                            obj.container()
                                .as_ref()
                                .map(model::Container::health_status)
                                .map(|status| status != model::ContainerHealthStatus::Unconfigured)
                                .unwrap_or(false),
                        );
                    }),
                );

            container_expr
                .chain_property::<model::Container>("image")
                .watch(
                    Some(obj),
                    clone!(@weak obj => move || {
                        obj.action_set_enabled(
                            ACTION_SHOW_IMAGE_DETAILS,
                            obj.container().as_ref().and_then(model::Container::image).is_some()
                        );
                    }),
                );
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ContainerDetailsPage {}

    impl ContainerDetailsPage {
        pub(crate) fn set_container(&self, value: Option<&model::Container>) {
            let obj = &*self.obj();
            if obj.container().as_ref() == value {
                return;
            }

            if let Some(container) = obj.container() {
                container.disconnect(self.handler_id.take().unwrap());
            }

            if let Some(container) = value {
                container.inspect(clone!(@weak obj => move |result| if let Err(e) = result {
                    utils::show_error_toast(obj.upcast_ref(), &gettext("Error on loading container details"), &e.to_string());
                }));

                let handler_id = container.connect_deleted(clone!(@weak obj => move |container| {
                    utils::show_toast(obj.upcast_ref(), gettext!("Container '{}' has been deleted", container.name()));
                    utils::navigation_view(obj.upcast_ref()).pop();
                }));
                self.handler_id.replace(Some(handler_id));

                let sorter = gtk::StringSorter::new(Some(
                    model::ContainerVolume::this_expression("volume")
                        .chain_property::<model::Volume>("inner")
                        .chain_closure::<String>(closure!(
                            |_: model::ContainerVolume, inner: model::BoxedVolume| {
                                inner.name.clone()
                            }
                        )),
                ));
                let model = gtk::SortListModel::new(Some(container.volume_list()), Some(sorter));

                self.volumes_list_box.bind_model(Some(&model), |item| {
                    view::ContainerVolumeRow::from(item.downcast_ref().unwrap()).upcast()
                });

                obj.update_volumes_visibility();
                container.volume_list().connect_items_changed(
                    clone!(@weak obj => move |_, _, _, _| obj.update_volumes_visibility()),
                );
            }

            self.container.set(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerDetailsPage(ObjectSubclass<imp::ContainerDetailsPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for ContainerDetailsPage {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder()
            .property("container", container)
            .build()
    }
}

impl ContainerDetailsPage {
    fn update_volumes_visibility(&self) {
        let imp = self.imp();
        imp.volumes_group
            .set_visible(imp.volumes_list_box.row_at_index(0).is_some());
    }

    fn update_actions(&self) {
        if let Some(container) = self.container() {
            let imp = self.imp();

            imp.action_row.set_sensitive(!container.action_ongoing());

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

    pub(crate) fn rename(&self) {
        self.exec_action(|| {
            if let Some(container) = self.container() {
                let dialog = view::ContainerRenameDialog::from(&container);
                dialog.set_transient_for(Some(&utils::root(self.upcast_ref())));
                dialog.present();
            }
        });
    }

    pub(crate) fn commit(&self) {
        self.exec_action(|| {
            if let Some(container) = self.container() {
                utils::show_dialog(
                    self.upcast_ref(),
                    view::ContainerCommitPage::from(&container).upcast_ref(),
                );
            }
        });
    }

    pub(crate) fn get_files(&self) {
        self.exec_action(|| {
            if let Some(container) = self.container() {
                utils::show_dialog(
                    self.upcast_ref(),
                    view::ContainerFilesGetPage::from(&container).upcast_ref(),
                );
            }
        });
    }

    pub(crate) fn put_files(&self) {
        self.exec_action(|| {
            if let Some(container) = self.container() {
                utils::show_dialog(
                    self.upcast_ref(),
                    view::ContainerFilesPutPage::from(&container).upcast_ref(),
                );
            }
        });
    }

    pub(crate) fn show_health_details(&self) {
        self.exec_action(|| {
            if let Some(ref container) = self.container() {
                utils::navigation_view(self.upcast_ref()).push(
                    &adw::NavigationPage::builder()
                        .child(&view::ContainerHealthCheckPage::from(container))
                        .build(),
                );
            }
        });
    }

    pub(crate) fn show_image_details(&self) {
        self.exec_action(|| {
            if let Some(image) = self.container().as_ref().and_then(model::Container::image) {
                utils::navigation_view(self.upcast_ref()).push(
                    &adw::NavigationPage::builder()
                        .child(&view::ImageDetailsPage::from(&image))
                        .build(),
                );
            }
        });
    }

    pub(crate) fn show_pod_details(&self) {
        self.exec_action(|| {
            if let Some(pod) = self.container().as_ref().and_then(model::Container::pod) {
                utils::navigation_view(self.upcast_ref()).push(
                    &adw::NavigationPage::builder()
                        .child(&view::PodDetailsPage::from(&pod))
                        .build(),
                );
            }
        });
    }

    pub(crate) fn show_inspection(&self) {
        self.show_kube_inspection_or_kube(view::ScalableTextViewMode::Inspect);
    }

    pub(crate) fn show_kube(&self) {
        self.show_kube_inspection_or_kube(view::ScalableTextViewMode::Kube);
    }

    pub(crate) fn show_kube_inspection_or_kube(&self, mode: view::ScalableTextViewMode) {
        self.exec_action(|| {
            if let Some(container) = self.container() {
                let weak_ref = glib::WeakRef::new();
                weak_ref.set(Some(&container));

                utils::navigation_view(self.upcast_ref()).push(
                    &adw::NavigationPage::builder()
                        .child(&view::ScalableTextViewPage::from(view::Entity::Container {
                            container: weak_ref,
                            mode,
                        }))
                        .build(),
                );
            }
        });
    }

    pub(crate) fn show_log(&self) {
        self.exec_action(|| {
            if let Some(container) = self.container() {
                utils::navigation_view(self.upcast_ref()).push(
                    &adw::NavigationPage::builder()
                        .child(&view::ContainerLogPage::from(&container))
                        .build(),
                );
            }
        });
    }

    pub(crate) fn show_processes(&self) {
        self.exec_action(|| {
            if let Some(container) = self.container() {
                utils::navigation_view(self.upcast_ref()).push(
                    &adw::NavigationPage::builder()
                        .child(&view::TopPage::from(&container))
                        .build(),
                );
            }
        });
    }

    pub(crate) fn show_tty(&self) {
        self.exec_action(|| {
            if let Some(container) = self.container() {
                utils::navigation_view(self.upcast_ref()).push(
                    &adw::NavigationPage::builder()
                        .child(&view::ContainerTerminalPage::from(&container))
                        .build(),
                );
            }
        });
    }

    fn exec_action<F: Fn()>(&self, op: F) {
        if utils::navigation_view(self.upcast_ref())
            .visible_page()
            .filter(|page| page.child().as_ref() == Some(self.upcast_ref()))
            .is_some()
        {
            op();
        }
    }
}
