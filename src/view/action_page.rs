use std::cell::Cell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use gtk::CompositeTemplate;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_CANCEL: &str = "action-page.cancel";
const ACTION_VIEW_OUTPUT: &str = "action-page.view-output";
const ACTION_VIEW_ARTIFACT: &str = "action-page.view-artifact";
const ACTION_RETRY: &str = "action-page.retry";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ActionPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/action_page.ui")]
    pub(crate) struct ActionPage {
        #[property(get, set, construct_only, nullable)]
        pub(super) action: glib::WeakRef<model::Action>,
        #[property(get, set, construct_only)]
        pub(super) show_view_artifact: Cell<bool>,
        #[template_child]
        pub(super) status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) view_artifact_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) output_box: TemplateChild<adw::Clamp>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ActionPage {
        const NAME: &'static str = "PdsActionPage";
        type Type = super::ActionPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action(ACTION_CANCEL, None, |widget, _, _| widget.cancel());
            klass.install_action(ACTION_VIEW_OUTPUT, None, |widget, _, _| {
                widget.view_output()
            });
            klass.install_action(ACTION_VIEW_ARTIFACT, None, |widget, _, _| {
                widget.view_artifact();
            });
            klass.install_action(ACTION_RETRY, None, |widget, _, _| widget.retry());
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ActionPage {
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
            use model::ActionType::*;

            self.parent_constructed();

            let obj = &*self.obj();

            let action = obj.action().unwrap();

            obj.update_state(&action);
            action.connect_notify_local(
                Some("state"),
                clone!(
                    #[weak]
                    obj,
                    move |action, _| obj.update_state(action)
                ),
            );

            self.status_page
                .set_icon_name(Some(match action.action_type() {
                    PruneContainers | PruneImages | PrunePods | PruneVolumes => "eraser5-symbolic",
                    DownloadImage | BuildImage => "image-x-generic-symbolic",
                    PushImage => "put-symbolic",
                    Commit => "merge-symbolic",
                    CreateContainer => "package-x-generic-symbolic",
                    CreateAndRunContainer => "media-playback-start-symbolic",
                    CopyFiles => "edit-copy-symbolic",
                    Pod => "pods-symbolic",
                    Volume => "drive-harddisk-symbolic",
                    _ => unimplemented!(),
                }));

            obj.set_description(&action);
            glib::timeout_add_seconds_local(
                1,
                clone!(
                    #[weak]
                    obj,
                    #[weak]
                    action,
                    #[upgrade_or]
                    glib::ControlFlow::Break,
                    move || obj.set_description(&action)
                ),
            );
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ActionPage {}
}

glib::wrapper! {
    pub(crate) struct ActionPage(ObjectSubclass<imp::ActionPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Action> for ActionPage {
    fn from(action: &model::Action) -> Self {
        Self::new(action, true)
    }
}

impl ActionPage {
    pub(crate) fn new(action: &model::Action, show_view_artifact: bool) -> Self {
        glib::Object::builder()
            .property("action", action)
            .property("show-view-artifact", show_view_artifact)
            .build()
    }

    fn update_state(&self, action: &model::Action) {
        use model::ActionState::*;
        use model::ActionType::*;

        let imp = self.imp();

        match action.state() {
            model::ActionState::Ongoing => {
                imp.status_page.set_title(&match action.action_type() {
                    PruneImages => gettext("Pruning Images"),
                    DownloadImage => gettext("Downloading Image"),
                    BuildImage => gettext("Building Image"),
                    PushImage => gettext("Pushing Image"),
                    PruneContainers => gettext("Pruning Containers"),
                    CreateContainer => gettext("Creating Container"),
                    CreateAndRunContainer => gettext("Starting Container"),
                    Commit => gettext("Committing Image"),
                    CopyFiles => gettext("Copying Files"),
                    PrunePods => gettext("Pruning Pods"),
                    Pod => gettext("Creating Pod"),
                    Volume => gettext("Creating Volume"),
                    PruneVolumes => gettext("Pruning Volumes"),
                    _ => unreachable!(),
                });
            }
            Finished => {
                imp.status_page.set_title(&match action.action_type() {
                    PruneImages => gettext("Images Pruned"),
                    DownloadImage => gettext("Image Downloaded"),
                    BuildImage => gettext("Image Built"),
                    PushImage => gettext("Image Pushed"),
                    PruneContainers => gettext("Containers Pruned"),
                    CreateContainer => gettext("Container Created"),
                    CreateAndRunContainer => gettext("Container Started"),
                    Commit => gettext("Image Committed"),
                    CopyFiles => gettext("Files Copied"),
                    PrunePods => gettext("Pods Pruned"),
                    Pod => gettext("Pod Created"),
                    Volume => gettext("Volume Created"),
                    PruneVolumes => gettext("Volumes Pruned"),
                    _ => unreachable!(),
                });
                imp.view_artifact_button
                    .set_label(&match action.action_type() {
                        DownloadImage => gettext("View Image"),
                        BuildImage => gettext("View Image"),
                        CreateContainer => gettext("View Container"),
                        CreateAndRunContainer => gettext("View Container"),
                        Pod => gettext("View Pod"),
                        Volume => gettext("View Volume"),
                        _ => unreachable!(),
                    });
            }
            Aborted => {
                imp.status_page.set_title(&match action.action_type() {
                    PruneImages => gettext("Image Pruning Aborted"),
                    DownloadImage => gettext("Image Download Aborted"),
                    BuildImage => gettext("Image Built Aborted"),
                    PushImage => gettext("Image Push Aborted"),
                    PruneContainers => gettext("Container Pruning Aborted"),
                    CreateContainer => gettext("Container Creation Aborted"),
                    CreateAndRunContainer => gettext("Container Start Aborted"),
                    Commit => gettext("Image Commit Aborted"),
                    CopyFiles => gettext("File Copying Aborted"),
                    PrunePods => gettext("Pod Pruning Aborted"),
                    Pod => gettext("Pod Creation Aborted"),
                    Volume => gettext("Volume Creation Aborted"),
                    PruneVolumes => gettext("Volume Pruning Aborted"),
                    _ => unreachable!(),
                });
            }
            Failed => {
                imp.status_page.set_title(&match action.action_type() {
                    PruneImages => gettext("Pruning Image Failed"),
                    DownloadImage => gettext("Downloading Image Failed"),
                    BuildImage => gettext("Building Image Failed"),
                    PushImage => gettext("Pushing Image Failed"),
                    PruneContainers => gettext("Pruning Containers Failed"),
                    CreateContainer => gettext("Creating Container Failed"),
                    CreateAndRunContainer => gettext("Starting Container Failed"),
                    Commit => gettext("Committing Image Failed"),
                    CopyFiles => gettext("Copying Files Failed"),
                    PrunePods => gettext("Pruning Pods Failed"),
                    Pod => gettext("Creating Pod Failed"),
                    Volume => gettext("Creating Volume Failed"),
                    PruneVolumes => gettext("Pruning Volumes Failed"),
                    _ => unreachable!(),
                });
            }
        }

        self.set_description(action);

        self.action_set_enabled(ACTION_CANCEL, action.state() == Ongoing);
        self.action_set_enabled(
            ACTION_VIEW_ARTIFACT,
            self.show_view_artifact()
                && action.state() == Finished
                && !matches!(
                    action.action_type(),
                    PruneContainers
                        | PruneImages
                        | PrunePods
                        | PruneVolumes
                        | Commit
                        | CopyFiles
                        | PushImage
                ),
        );
        self.action_set_enabled(
            ACTION_RETRY,
            matches!(action.state(), Aborted | Failed)
                && self.ancestor(gtk::Stack::static_type()).is_some(),
        );
    }

    fn set_description(&self, action: &model::Action) -> glib::ControlFlow {
        let state_label = &*self.imp().status_page;

        match action.state() {
            model::ActionState::Ongoing => {
                state_label.set_description(Some(&utils::human_friendly_duration(
                    glib::DateTime::now_local().unwrap().to_unix() - action.start_timestamp(),
                )));

                glib::ControlFlow::Continue
            }
            _ => {
                state_label.set_description(Some(&gettext!(
                    "After {}",
                    utils::human_friendly_duration(
                        action.end_timestamp() - action.start_timestamp(),
                    )
                )));

                glib::ControlFlow::Break
            }
        }
    }

    fn cancel(&self) {
        if let Some(action) = self.action() {
            action.cancel();
        }
    }

    fn view_output(&self) {
        let imp = self.imp();
        imp.status_page.set_icon_name(None);
        imp.status_page.set_description(None);
        imp.output_box.set_visible(true);
        self.action_set_enabled(ACTION_VIEW_OUTPUT, false);
    }

    fn view_artifact(&self) {
        match self.action().as_ref().and_then(model::Action::artifact) {
            Some(artifact) => {
                let page = if let Some(image) = artifact.downcast_ref::<model::Image>() {
                    view::ImageDetailsPage::from(image).upcast::<gtk::Widget>()
                } else if let Some(container) = artifact.downcast_ref::<model::Container>() {
                    view::ContainerDetailsPage::from(container).upcast()
                } else if let Some(pod) = artifact.downcast_ref::<model::Pod>() {
                    view::PodDetailsPage::from(pod).upcast()
                } else if let Some(volume) = artifact.downcast_ref::<model::Volume>() {
                    view::VolumeDetailsPage::from(volume).upcast()
                } else {
                    unreachable!();
                };

                gio::Application::default()
                    .unwrap()
                    .downcast::<crate::Application>()
                    .unwrap()
                    .main_window()
                    .navigation_view()
                    .push(
                        &adw::NavigationPage::builder()
                            .title(gettext("Action"))
                            .child(&page)
                            .build(),
                    );

                self.activate_action("win.close", None).unwrap();
            }
            None => utils::show_error_toast(
                self,
                &gettext("Error on opening artifact"),
                &gettext("Artifact has been deleted"),
            ),
        }
    }

    fn retry(&self) {
        if let Some(stack) = self
            .ancestor(gtk::Stack::static_type())
            .and_then(|ancestor| ancestor.downcast::<gtk::Stack>().ok())
        {
            stack.set_visible_child(&stack.first_child().unwrap());
        }
    }
}
