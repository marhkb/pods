use std::cell::RefCell;

use gettextrs::gettext;
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

const ACTION_START_OR_RESUME: &str = "container-details-page.start";
const ACTION_STOP: &str = "container-details-page.stop";
const ACTION_KILL: &str = "container-details-page.kill";
const ACTION_RESTART: &str = "container-details-page.restart";
const ACTION_PAUSE: &str = "container-details-page.pause";
const ACTION_RESUME: &str = "container-details-page.resume";
const ACTION_DELETE: &str = "container-details-page.delete";

const ACTION_INSPECT: &str = "container-details-page.inspect";
const ACTION_GENERATE_KUBE: &str = "container-details-page.generate-kube";
const ACTION_SHOW_LOG: &str = "container-details-page.show-log";
const ACTION_SHOW_PROCESSES: &str = "container-details-page.show-processes";
const ACTION_SHOW_COMMIT_PAGE: &str = "container-details-page.show-commit-page";
const ACTION_GET_FILES: &str = "container-details-page.get-files";
const ACTION_PUT_FILES: &str = "container-details-page.put-files";

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
            Self::bind_template(klass);

            klass.install_action(ACTION_INSPECT, None, move |widget, _, _| {
                widget.show_inspection();
            });
            klass.install_action(ACTION_GENERATE_KUBE, None, move |widget, _, _| {
                widget.show_kube();
            });
            klass.install_action(ACTION_SHOW_LOG, None, move |widget, _, _| {
                widget.show_log();
            });
            klass.install_action(ACTION_SHOW_PROCESSES, None, move |widget, _, _| {
                widget.show_processes();
            });
            klass.install_action(ACTION_SHOW_COMMIT_PAGE, None, move |widget, _, _| {
                widget.show_commit_page();
            });
            klass.install_action_async(ACTION_GET_FILES, None, move |widget, _, _| async move {
                widget.get_files().await;
            });
            klass.install_action(ACTION_PUT_FILES, None, move |widget, _, _| {
                widget.put_files();
            });

            klass.install_action(ACTION_START_OR_RESUME, None, move |widget, _, _| {
                if widget.container().map(|c| c.can_start()).unwrap_or(false) {
                    super::super::start(widget.upcast_ref());
                } else {
                    super::super::resume(widget.upcast_ref());
                }
            });
            klass.install_action(ACTION_STOP, None, move |widget, _, _| {
                super::super::stop(widget.upcast_ref());
            });
            klass.install_action(ACTION_KILL, None, move |widget, _, _| {
                super::super::kill(widget.upcast_ref());
            });
            klass.install_action(ACTION_RESTART, None, move |widget, _, _| {
                super::super::restart(widget.upcast_ref());
            });
            klass.install_action(ACTION_PAUSE, None, move |widget, _, _| {
                super::super::pause(widget.upcast_ref());
            });
            klass.install_action(ACTION_RESUME, None, move |widget, _, _| {
                super::super::resume(widget.upcast_ref());
            });
            klass.install_action(ACTION_DELETE, None, move |widget, _, _| {
                super::super::delete(widget.upcast_ref());
            });

            klass.install_action("container.tty", None, move |widget, _, _| {
                widget.show_tty();
            });
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
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
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
    fn from(image: &model::Container) -> Self {
        glib::Object::builder::<Self>()
            .property("container", image)
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
                utils::show_error_toast(&obj, &gettext("Error on loading container details"), &e.to_string());
            }));

            let handler_id = container.connect_deleted(clone!(@weak self as obj => move |container| {
                utils::show_toast(&obj, &gettext!("Container '{}' has been deleted", container.name()));
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
            self.action_set_enabled(ACTION_RESTART, container.can_restart());
            self.action_set_enabled(ACTION_PAUSE, container.can_pause());
            self.action_set_enabled(ACTION_DELETE, container.can_delete());
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

            self.imp()
                .leaflet_overlay
                .show_details(&view::SourceViewPage::from(view::Entity::Container {
                    container: weak_ref,
                    mode,
                }));
        }
    }

    fn show_log(&self) {
        if let Some(container) = self.container() {
            self.imp()
                .leaflet_overlay
                .show_details(&view::ContainerLogPage::from(&container));
        }
    }

    fn show_processes(&self) {
        if let Some(container) = self.container() {
            self.imp()
                .leaflet_overlay
                .show_details(&view::TopPage::from(&container));
        }
    }

    fn show_commit_page(&self) {
        if let Some(container) = self.container() {
            self.imp()
                .leaflet_overlay
                .show_details(&view::ContainerCommitPage::from(&container));
        }
    }

    async fn get_files(&self) {
        if let Some(container) = self.container() {
            self.imp()
                .leaflet_overlay
                .show_details(&view::ContainerFilesGetPage::from(&container));
        }
    }

    fn put_files(&self) {
        if let Some(container) = self.container() {
            self.imp()
                .leaflet_overlay
                .show_details(&view::ContainerFilesPutPage::from(&container));
        }
    }

    fn show_tty(&self) {
        if let Some(container) = self.container() {
            self.imp()
                .leaflet_overlay
                .show_details(&view::ContainerTtyPage::from(&container));
        }
    }
}
