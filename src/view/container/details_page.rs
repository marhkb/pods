use std::cell::RefCell;

use gettextrs::gettext;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

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

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container/details-page.ui")]
    pub(crate) struct DetailsPage {
        pub(super) container: glib::WeakRef<model::Container>,
        pub(super) handler_id: RefCell<Option<glib::SignalHandlerId>>,
        #[template_child]
        pub(super) back_navigation_controls: TemplateChild<view::BackNavigationControls>,
        #[template_child]
        pub(super) action_row: TemplateChild<adw::PreferencesRow>,
        #[template_child]
        pub(super) start_or_resume_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) stop_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) spinning_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) resources_quick_reference_group:
            TemplateChild<view::ContainerResourcesQuickReferenceGroup>,
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DetailsPage {
        const NAME: &'static str = "PdsContainerDetailsPage";
        type Type = super::DetailsPage;
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
                    super::super::start(widget.upcast_ref());
                } else {
                    super::super::resume(widget.upcast_ref());
                }
            });
            klass.install_action(ACTION_STOP, None, |widget, _, _| {
                super::super::stop(widget.upcast_ref());
            });
            klass.install_action(ACTION_KILL, None, |widget, _, _| {
                super::super::kill(widget.upcast_ref());
            });
            klass.install_action(ACTION_RESTART, None, |widget, _, _| {
                super::super::restart(widget.upcast_ref());
            });
            klass.install_action(ACTION_PAUSE, None, |widget, _, _| {
                super::super::pause(widget.upcast_ref());
            });
            klass.install_action(ACTION_RESUME, None, |widget, _, _| {
                super::super::resume(widget.upcast_ref());
            });
            klass.install_action(ACTION_DELETE, None, |widget, _, _| {
                super::super::delete(widget.upcast_ref());
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

    impl ObjectImpl for DetailsPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Container>("container")
                        .construct()
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
            let status_expr = container_expr.chain_property::<model::Container>("status");

            status_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, status: model::ContainerStatus| {
                    status == model::ContainerStatus::Running
                }))
                .bind(&*self.resources_quick_reference_group, "visible", Some(obj));

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
            utils::ChildIter::from(self.obj().upcast_ref()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for DetailsPage {}
}

glib::wrapper! {
    pub(crate) struct DetailsPage(ObjectSubclass<imp::DetailsPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for DetailsPage {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder()
            .property("container", container)
            .build()
    }
}

impl DetailsPage {
    fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    fn set_container(&self, value: Option<&model::Container>) {
        if self.container().as_ref() == value {
            return;
        }

        let imp = self.imp();

        if let Some(container) = self.container() {
            container.disconnect(imp.handler_id.take().unwrap());
        }

        if let Some(container) = value {
            container.inspect(clone!(@weak self as obj => move |e| {
                utils::show_error_toast(obj.upcast_ref(), &gettext("Error on loading container details"), &e.to_string());
            }));

            let handler_id = container.connect_deleted(clone!(@weak self as obj => move |container| {
                utils::show_toast(obj.upcast_ref(), &gettext!("Container '{}' has been deleted", container.name()));
                obj.imp().back_navigation_controls.navigate_back();
            }));
            imp.handler_id.replace(Some(handler_id));
        }

        imp.container.set(value);
        self.notify("container");
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

    fn rename(&self) {
        if let Some(container) = self.container() {
            let dialog = view::ContainerRenameDialog::from(&container);
            dialog.set_transient_for(Some(&utils::root(self.upcast_ref())));
            dialog.present();
        }
    }

    fn commit(&self) {
        if let Some(container) = self.container() {
            utils::show_dialog(
                self.upcast_ref(),
                view::ContainerCommitPage::from(&container).upcast_ref(),
            );
        }
    }

    fn get_files(&self) {
        if let Some(container) = self.container() {
            utils::show_dialog(
                self.upcast_ref(),
                view::ContainerFilesGetPage::from(&container).upcast_ref(),
            );
        }
    }

    fn put_files(&self) {
        if let Some(container) = self.container() {
            utils::show_dialog(
                self.upcast_ref(),
                view::ContainerFilesPutPage::from(&container).upcast_ref(),
            );
        }
    }

    fn show_health_details(&self) {
        if let Some(ref container) = self.container() {
            self.imp()
                .leaflet_overlay
                .show_details(view::ContainerHealthCheckPage::from(container).upcast_ref());
        }
    }

    fn show_image_details(&self) {
        if let Some(image) = self.container().as_ref().and_then(model::Container::image) {
            self.imp()
                .leaflet_overlay
                .show_details(view::ImageDetailsPage::from(&image).upcast_ref());
        }
    }

    fn show_pod_details(&self) {
        if let Some(pod) = self.container().as_ref().and_then(model::Container::pod) {
            self.imp()
                .leaflet_overlay
                .show_details(view::PodDetailsPage::from(&pod).upcast_ref());
        }
    }

    fn show_inspection(&self) {
        self.show_kube_inspection_or_kube(view::SourceViewMode::Inspect);
    }

    fn show_kube(&self) {
        self.show_kube_inspection_or_kube(view::SourceViewMode::Kube);
    }

    fn show_kube_inspection_or_kube(&self, mode: view::SourceViewMode) {
        if let Some(container) = self.container() {
            let weak_ref = glib::WeakRef::new();
            weak_ref.set(Some(&container));

            self.imp().leaflet_overlay.show_details(
                view::SourceViewPage::from(view::Entity::Container {
                    container: weak_ref,
                    mode,
                })
                .upcast_ref(),
            );
        }
    }

    fn show_log(&self) {
        if let Some(container) = self.container() {
            self.imp()
                .leaflet_overlay
                .show_details(view::ContainerLogPage::from(&container).upcast_ref());
        }
    }

    fn show_processes(&self) {
        if let Some(container) = self.container() {
            self.imp()
                .leaflet_overlay
                .show_details(view::TopPage::from(&container).upcast_ref());
        }
    }

    fn show_tty(&self) {
        if let Some(container) = self.container() {
            self.imp()
                .leaflet_overlay
                .show_details(view::ContainerTtyPage::from(&container).upcast_ref());
        }
    }
}
