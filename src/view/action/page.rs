use adw::subclass::prelude::*;
use adw::traits::BinExt;
use gettextrs::gettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_CANCEL: &str = "action-page.cancel";
const ACTION_VIEW_IMAGE: &str = "action-page.view-artifact";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/action/page.ui")]
    pub(crate) struct Page {
        pub(super) action: glib::WeakRef<model::Action>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) artifact_page_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Page {
        const NAME: &'static str = "PdsActionPage";
        type Type = super::Page;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action(ACTION_CANCEL, None, |widget, _, _| widget.cancel());
            klass.install_action(ACTION_VIEW_IMAGE, None, move |widget, _, _| {
                widget.view_artifact();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Page {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "action",
                    "Action",
                    "The action of this image pulling page",
                    model::Action::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "action" => self.action.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "action" => self.instance().action().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            use model::ActionType::*;

            self.parent_constructed();

            let obj = &*self.instance();

            let action = obj.action().unwrap();

            obj.update_state(&action);
            action.connect_notify_local(
                Some("state"),
                clone!(@weak obj => move |action, _| obj.update_state(action)),
            );

            self.status_page.set_icon_name(Some(match action.type_() {
                PruneImages => "larger-brush-symbolic",
                DownloadImage | BuildImage => "image-x-generic-symbolic",
                Commit => "merge-symbolic",
                Container => "package-x-generic-symbolic",
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
            utils::ChildIter::from(&*self.instance()).for_each(|child| child.unparent());
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
        glib::Object::new::<Self>(&[("action", &action)])
    }
}

impl Page {
    fn action(&self) -> Option<model::Action> {
        self.imp().action.upgrade()
    }

    fn update_state(&self, action: &model::Action) {
        use model::ActionState::*;
        use model::ActionType::*;

        let imp = self.imp();

        match action.state() {
            model::ActionState::Ongoing => {
                imp.status_page.set_title(&match action.type_() {
                    PruneImages => gettext("Images Are Currently Being Pruned"),
                    DownloadImage => gettext("Image Is Currently Being Downloaded"),
                    BuildImage => gettext("Image Is Currently Being Built"),
                    Container => gettext("Container Is Currently Being Created"),
                    Commit => gettext("New Image Is Currently Being Commited"),
                    Pod => gettext("Pod Is Currently Being Created"),
                    _ => unreachable!(),
                });
            }
            Finished => {
                imp.status_page.set_title(&match action.type_() {
                    PruneImages => gettext("Images Have Been Pruned"),
                    DownloadImage => gettext("Image Has Been Downloaded"),
                    BuildImage => gettext("Image Has Been Built"),
                    Container => gettext("Container Has Been Created"),
                    Commit => gettext("New Image Has Been Commited"),
                    Pod => gettext("Pod Has Been Created"),
                    _ => unreachable!(),
                });
            }
            Cancelled => {
                imp.status_page.set_title(&match action.type_() {
                    PruneImages => gettext("Pruning of Images Has Been Aborted"),
                    DownloadImage => gettext("Image Download Has Been Aborted"),
                    BuildImage => gettext("Image Built Has Been Aborted"),
                    Container => gettext("Container Creation Has Been Aborted"),
                    Commit => gettext("Image Commitment Has Been Aborted"),
                    Pod => gettext("Pod Creation Has Been Aborted"),
                    _ => unreachable!(),
                });
            }
            Failed => {
                imp.status_page.set_title(&match action.type_() {
                    PruneImages => gettext("Pruning of Images Has Failed"),
                    DownloadImage => gettext("Image Download Has Failed"),
                    BuildImage => gettext("Image Built Has Failed"),
                    Container => gettext("Container Creation Has Failed"),
                    Commit => gettext("Image Commitment Has Failed"),
                    Pod => gettext("Pod Creation Has Failed"),
                    _ => unreachable!(),
                });
            }
        }

        self.set_description(action);

        self.action_set_enabled(ACTION_CANCEL, action.state() == Ongoing);
        self.action_set_enabled(
            ACTION_VIEW_IMAGE,
            action.state() == Finished && !matches!(action.type_(), PruneImages | Commit),
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
                let imp = self.imp();

                imp.artifact_page_bin.set_child(Some(&if let Some(image) =
                    artifact.downcast_ref::<model::Image>()
                {
                    view::ImageDetailsPage::from(image).upcast::<gtk::Widget>()
                } else if let Some(container) = artifact.downcast_ref::<model::Container>() {
                    view::ContainerDetailsPage::from(container).upcast()
                } else if let Some(pod) = artifact.downcast_ref::<model::Pod>() {
                    view::PodDetailsPage::from(pod).upcast()
                } else {
                    unreachable!();
                }));

                imp.main_stack.set_visible_child(&*imp.artifact_page_bin);
            }
            None => utils::show_error_toast(
                self,
                &gettext("Error on opening artifact"),
                &gettext("Artifact has been deleted"),
            ),
        }
    }
}
