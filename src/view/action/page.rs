use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_CANCEL: &str = "action-page.cancel";
const ACTION_VIEW_ARTIFACT: &str = "action-page.view-artifact";
const ACTION_RETRY: &str = "action-page.retry";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::Page)]
    #[template(resource = "/com/github/marhkb/Pods/ui/action/page.ui")]
    pub(crate) struct Page {
        #[property(get, set, construct_only, nullable)]
        pub(super) action: glib::WeakRef<model::Action>,
        #[template_child]
        pub(super) status_page: TemplateChild<adw::StatusPage>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Page {
        const NAME: &'static str = "PdsActionPage";
        type Type = super::Page;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action(ACTION_CANCEL, None, |widget, _, _| widget.cancel());
            klass.install_action(ACTION_VIEW_ARTIFACT, None, |widget, _, _| {
                widget.view_artifact();
            });
            klass.install_action(ACTION_RETRY, None, |widget, _, _| widget.retry());
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Page {
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
                clone!(@weak obj => move |action, _| obj.update_state(action)),
            );

            self.status_page
                .set_icon_name(Some(match action.action_type() {
                    PruneImages => "eraser5-symbolic",
                    DownloadImage | BuildImage => "image-x-generic-symbolic",
                    Commit => "merge-symbolic",
                    Container => "package-x-generic-symbolic",
                    CopyFiles => "edit-copy-symbolic",
                    Pod => "pods-symbolic",
                    _ => unimplemented!(),
                }));

            obj.set_description(&action);
            glib::timeout_add_seconds_local(
                1,
                clone!(@weak obj, @weak action => @default-return glib::Continue(false), move || {
                    glib::Continue(obj.set_description(&action))
                }),
            );
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for Page {}
}

glib::wrapper! {
    pub(crate) struct Page(ObjectSubclass<imp::Page>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Action> for Page {
    fn from(action: &model::Action) -> Self {
        glib::Object::builder().property("action", action).build()
    }
}

impl Page {
    fn update_state(&self, action: &model::Action) {
        use model::ActionState::*;
        use model::ActionType::*;

        let imp = self.imp();

        match action.state() {
            model::ActionState::Ongoing => {
                imp.status_page.set_title(&match action.action_type() {
                    PruneImages => gettext("Images Are Currently Being Pruned"),
                    DownloadImage => gettext("Image Is Currently Being Downloaded"),
                    BuildImage => gettext("Image Is Currently Being Built"),
                    Container => gettext("Container Is Currently Being Created"),
                    Commit => gettext("New Image Is Currently Being Committed"),
                    CopyFiles => gettext("Files Are Currently Being Copied"),
                    Pod => gettext("Pod Is Currently Being Created"),
                    _ => unreachable!(),
                });
            }
            Finished => {
                imp.status_page.set_title(&match action.action_type() {
                    PruneImages => gettext("Images Have Been Pruned"),
                    DownloadImage => gettext("Image Has Been Downloaded"),
                    BuildImage => gettext("Image Has Been Built"),
                    Container => gettext("Container Has Been Created"),
                    Commit => gettext("New Image Has Been Committed"),
                    CopyFiles => gettext("Files Have Beeng Copied"),
                    Pod => gettext("Pod Has Been Created"),
                    _ => unreachable!(),
                });
            }
            Cancelled => {
                imp.status_page.set_title(&match action.action_type() {
                    PruneImages => gettext("Pruning of Images Has Been Aborted"),
                    DownloadImage => gettext("Image Download Has Been Aborted"),
                    BuildImage => gettext("Image Built Has Been Aborted"),
                    Container => gettext("Container Creation Has Been Aborted"),
                    Commit => gettext("Image Commitment Has Been Aborted"),
                    CopyFiles => gettext("Copying Files Has Been Aborted"),
                    Pod => gettext("Pod Creation Has Been Aborted"),
                    _ => unreachable!(),
                });
            }
            Failed => {
                imp.status_page.set_title(&match action.action_type() {
                    PruneImages => gettext("Pruning of Images Has Failed"),
                    DownloadImage => gettext("Image Download Has Failed"),
                    BuildImage => gettext("Image Built Has Failed"),
                    Container => gettext("Container Creation Has Failed"),
                    Commit => gettext("Image Commitment Has Failed"),
                    CopyFiles => gettext("Copying Files Has Failed"),
                    Pod => gettext("Pod Creation Has Failed"),
                    _ => unreachable!(),
                });
            }
        }

        self.set_description(action);

        self.action_set_enabled(ACTION_CANCEL, action.state() == Ongoing);
        self.action_set_enabled(
            ACTION_VIEW_ARTIFACT,
            action.state() == Finished
                && !matches!(action.action_type(), PruneImages | Commit | CopyFiles),
        );
        self.action_set_enabled(
            ACTION_RETRY,
            matches!(action.state(), Cancelled | Failed)
                && self.ancestor(gtk::Stack::static_type()).is_some(),
        );
    }

    fn set_description(&self, action: &model::Action) -> bool {
        let state_label = &*self.imp().status_page;

        match action.state() {
            model::ActionState::Ongoing => {
                state_label.set_description(Some(&utils::human_friendly_duration(
                    glib::DateTime::now_local().unwrap().to_unix() - action.start_timestamp(),
                )));

                true
            }
            _ => {
                state_label.set_description(Some(&gettext!(
                    "After {}",
                    utils::human_friendly_duration(
                        action.end_timestamp() - action.start_timestamp(),
                    )
                )));

                false
            }
        }
    }

    fn cancel(&self) {
        if let Some(action) = self.action() {
            action.cancel();
        }
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
                } else {
                    unreachable!();
                };

                gio::Application::default()
                    .unwrap()
                    .downcast::<crate::Application>()
                    .unwrap()
                    .main_window()
                    .leaflet_overlay()
                    .show_details(&page);

                self.activate_action("action.cancel", None).unwrap();
            }
            None => utils::show_error_toast(
                self.upcast_ref(),
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
