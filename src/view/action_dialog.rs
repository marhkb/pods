use std::cell::OnceCell;
use std::marker::PhantomData;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::model::prelude::*;
use crate::utils;
use crate::view;

const ACTION_CANCEL: &str = "action-dialog.cancel";
const ACTION_VIEW_ARTIFACT: &str = "action-dialog.view-artifact";
const ACTION_EDIT: &str = "action-dialog.edit";

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ActionDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/action_dialog.ui")]
    pub(crate) struct ActionDialog {
        #[property(get, set = Self::set_action, construct_only, nullable, explicit_notify)]
        pub(super) action: glib::WeakRef<model::BaseAction>,

        #[property(get = Self::title, set, construct_only, nullable)]
        pub(super) title: OnceCell<Option<String>>,
        #[property(get = Self::subtitle, set, construct_only, nullable)]
        pub(super) subtitle: OnceCell<Option<String>>,
        #[property(get = Self::child, set, construct_only, nullable)]
        pub(super) child: OnceCell<Option<gtk::Widget>>,
        #[property(get = Self::produces_artifact)]
        _produces_artifact: PhantomData<bool>,

        #[template_child]
        pub(super) header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub(super) state_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) time_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ActionDialog {
        const NAME: &'static str = "PdsActionDialog";
        type Type = super::ActionDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_CANCEL, None, |widget, _, _| widget.cancel());
            klass.install_action(ACTION_VIEW_ARTIFACT, None, |widget, _, _| {
                widget.view_artifact();
            });
            klass.install_action(ACTION_EDIT, None, |widget, _, _| widget.edit());
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ActionDialog {
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

            let action_expr = Self::Type::this_expression("action");
            let action_state_expr = action_expr.chain_property::<model::BaseAction>("state");

            action_state_expr
                .chain_closure::<String>(closure!(|_: Self::Type, state: model::ActionState2| {
                    match state {
                        model::ActionState2::Cancelled => gettext("Cancelled"),
                        model::ActionState2::Failed => gettext("Failed"),
                        model::ActionState2::Finished => gettext("Finished"),
                        model::ActionState2::Ongoing => gettext("Ongoing"),
                    }
                }))
                .bind(&*self.state_label, "label", Some(obj));

            action_state_expr
                .chain_closure::<String>(closure!(|_: Self::Type, state: model::ActionState2| {
                    match state {
                        model::ActionState2::Ongoing => "ongoing",
                        _ => "finished",
                    }
                }))
                .bind(&*self.stack, "visible-child-name", Some(obj));
        }
    }

    impl WidgetImpl for ActionDialog {}
    impl AdwDialogImpl for ActionDialog {}

    impl ActionDialog {
        fn set_action(&self, action: Option<&model::BaseAction>) {
            let obj = &*self.obj();
            if obj.action().as_ref() == action {
                return;
            }

            if let Some(action) = action {
                action.connect_state_notify(clone!(
                    #[weak]
                    obj,
                    move |action| obj.update_view(action)
                ));

                if let Some(action) = action.downcast_ref::<model::ArtifactAction>() {
                    action.connect_artifact_notify(clone!(
                        #[weak]
                        obj,
                        move |action| obj.update_view(action.upcast_ref())
                    ));
                }

                obj.update_view(action);

                glib::timeout_add_seconds_local(
                    1,
                    clone!(
                        #[weak]
                        obj,
                        #[weak]
                        action,
                        #[upgrade_or]
                        glib::ControlFlow::Break,
                        move || obj.update_time_label(&action)
                    ),
                );

                obj.update_time_label(action);
            }

            self.action.set(action);
            obj.notify_action();
            obj.notify_produces_artifact();
        }

        fn title(&self) -> Option<String> {
            self.title.get().cloned().flatten()
        }

        fn subtitle(&self) -> Option<String> {
            self.subtitle.get().cloned().flatten()
        }

        fn child(&self) -> Option<gtk::Widget> {
            self.child.get().cloned().flatten()
        }

        fn produces_artifact(&self) -> bool {
            self.obj()
                .action()
                .map(|action| action.is::<model::ArtifactAction>())
                .unwrap_or(false)
        }
    }
}

glib::wrapper! {
    pub(crate) struct ActionDialog(ObjectSubclass<imp::ActionDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl From<model::BaseAction> for ActionDialog {
    fn from(value: model::BaseAction) -> Self {
        if let Some(action) = value.downcast_ref::<model::ContainerCreateAction>() {
            Self::from(action)
        } else if let Some(action) = value.downcast_ref::<model::ContainersPruneAction>() {
            Self::from(action)
        } else if let Some(action) = value.downcast_ref::<model::ImagePullAction>() {
            Self::from(action)
        } else if let Some(action) = value.downcast_ref::<model::ImagesPruneAction>() {
            Self::from(action)
        } else if let Some(action) = value.downcast_ref::<model::PodsPruneAction>() {
            Self::from(action)
        } else if let Some(action) = value.downcast_ref::<model::VolumesPruneAction>() {
            Self::from(action)
        } else {
            unreachable!()
        }
    }
}

impl ActionDialog {
    pub(crate) fn new<W>(
        action: &model::BaseAction,
        title: &str,
        subtitle: Option<&str>,
        child: &W,
        content_height: i32,
    ) -> Self
    where
        W: IsA<gtk::Widget>,
    {
        glib::Object::builder()
            .property("action", action)
            .property("title", title)
            .property("subtitle", subtitle)
            .property("child", child)
            .property("content-height", content_height)
            .build()
    }

    fn update_view(&self, action: &model::BaseAction) {
        let imp = self.imp();

        match action.state() {
            model::ActionState2::Cancelled => {
                imp.state_label.remove_css_class("error");
                imp.state_label.add_css_class("warning");
            }
            model::ActionState2::Failed => {
                imp.state_label.remove_css_class("warning");
                imp.state_label.add_css_class("error");
            }
            _ => {
                imp.state_label.remove_css_class("error");
                imp.state_label.remove_css_class("warning");
            }
        }

        self.update_time_label(action);

        self.action_set_enabled(
            ACTION_CANCEL,
            action.state() == model::ActionState2::Ongoing,
        );
        self.action_set_enabled(
            ACTION_VIEW_ARTIFACT,
            action.state() == model::ActionState2::Finished
                && action
                    .downcast_ref::<model::ArtifactAction>()
                    .and_then(|action| action.artifact())
                    .is_some(),
        );
        self.action_set_enabled(
            ACTION_EDIT,
            matches!(
                action.state(),
                model::ActionState2::Cancelled
                    | model::ActionState2::Failed
                    | model::ActionState2::Finished
            ),
        );
    }

    fn update_time_label(&self, action: &model::BaseAction) -> glib::ControlFlow {
        let label = &*self.imp().time_label;

        match action.state() {
            model::ActionState2::Ongoing => {
                label.set_label(&gettext!(
                    "since {}",
                    utils::human_friendly_duration(
                        glib::DateTime::now_local().unwrap().to_unix() - action.start_timestamp(),
                    )
                ));

                glib::ControlFlow::Continue
            }
            _ => {
                label.set_label(&gettext!(
                    "after {}",
                    utils::human_friendly_duration(
                        action.end_timestamp() - action.start_timestamp(),
                    )
                ));

                glib::ControlFlow::Break
            }
        }
    }

    fn cancel(&self) {
        if let Some(action) = self.action() {
            action.cancel();
        }
    }

    fn view_artifact(&self) {
        let Some(action) = self
            .action()
            .and_then(|action| action.downcast::<model::ArtifactAction>().ok())
        else {
            return;
        };

        let Some(artifact) = action.artifact() else {
            return;
        };

        let page: gtk::Widget = if let Some(container) = artifact.downcast_ref::<model::Container>()
        {
            view::ContainerDetailsPage::from(container).upcast()
        } else if let Some(image) = artifact.downcast_ref::<model::Image>() {
            view::ImageDetailsPage::from(image).upcast()
        } else {
            return;
        };

        self.close();

        utils::main_window().navigation_view().push(
            &adw::NavigationPage::builder()
                .title(gettext("Action"))
                .child(&page)
                .build(),
        );
    }

    fn edit(&self) {
        let Some(action) = self.action() else {
            return;
        };

        let Some(client) = action
            .action_list()
            .and_then(|action_list| action_list.client())
        else {
            return;
        };

        self.close();

        let dialog: adw::Dialog =
            if let Some(action) = action.downcast_ref::<model::ContainerCreateAction>() {
                view::ContainerCreateOptsDialog::new(&client, Some(action.opts())).upcast()
            } else if let Some(action) = action.downcast_ref::<model::ContainersPruneAction>() {
                view::ContainersPruneOptsDialog::new(&client, Some(action.opts())).upcast()
            } else if action.is::<model::PodsPruneAction>() {
                view::PodsPruneOptsDialog::from(&client).upcast()
            } else if let Some(action) = action.downcast_ref::<model::ImagePullAction>() {
                view::ImagePullOptsDialog::new(&client, Some(action.opts())).upcast()
            } else if let Some(action) = action.downcast_ref::<model::ImagesPruneAction>() {
                view::ImagesPruneOptsDialog::new(&client, Some(action.opts())).upcast()
            } else if let Some(action) = action.downcast_ref::<model::VolumesPruneAction>() {
                view::VolumesPruneOptsDialog::new(&client, Some(action.opts())).upcast()
            } else {
                unreachable!()
            };

        dialog.present(Some(self))
    }
}
