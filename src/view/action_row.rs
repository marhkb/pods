use std::cell::OnceCell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use ashpd::desktop as ashpd;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::model::prelude::*;
use crate::rt;
use crate::utils;

const ACTION_CANCEL_OR_DELETE: &str = "action-row.cancel-or-delete";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ActionRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/action_row.ui")]
    pub(crate) struct ActionRow {
        pub(super) notification_id: OnceCell<glib::GString>,
        pub(super) handler: RefCell<Option<glib::SignalHandlerId>>,
        pub(super) timer: RefCell<Option<glib::SourceId>>,
        #[property(get, set = Self::set_action, explicit_notify, nullable)]
        pub(super) action: glib::WeakRef<model::BaseAction>,
        #[template_child]
        pub(super) type_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) description_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) state_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) action_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ActionRow {
        const NAME: &'static str = "PdsActionRow";
        type Type = super::ActionRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.set_css_name("actionrow");

            klass.install_action(ACTION_CANCEL_OR_DELETE, None, |widget, _, _| {
                widget.cancel_or_delete();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ActionRow {
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

            self.notification_id
                .set(glib::uuid_string_random())
                .unwrap();

            let action_expr = Self::Type::this_expression("action");
            #[rustfmt::skip]
            let action_description_expr = action_expr.chain_closure::<String>(
                closure!(|_: Self::Type, action: Option<&model::BaseAction>| {
                    action
                        .map(action_description)
                        .unwrap_or_default()
                })
            );
            #[rustfmt::skip]
            let action_icon_name_expr = action_expr.chain_closure::<String>(
                closure!(|_: Self::Type, action: Option<&model::BaseAction>| {
                    action
                        .map(action_image)
                        .map(ToOwned::to_owned)
                        .unwrap_or_default()
                })
            );
            let action_state_expr = action_expr.chain_property::<model::BaseAction>("state");

            action_icon_name_expr.bind(&*self.type_image, "icon-name", Some(obj));
            action_description_expr.bind(obj, "tooltip-markup", Some(obj));
            action_description_expr.bind(&*self.description_label, "label", Some(obj));

            let classes = utils::css_classes(&*self.state_label);
            action_state_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, state: model::ActionState2| {
                        classes
                            .iter()
                            .cloned()
                            .chain(Some(String::from(match state {
                                model::ActionState2::Cancelled => "warning",
                                model::ActionState2::Failed => "error",
                                model::ActionState2::Finished => "dim-label",
                                model::ActionState2::Ongoing => "accent",
                            })))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.state_label, "css-classes", Some(obj));

            action_state_expr
                .chain_closure::<String>(closure!(|_: Self::Type, state: model::ActionState2| {
                    if state == model::ActionState2::Cancelled {
                        "window-close-symbolic"
                    } else {
                        "cross-symbolic"
                    }
                }))
                .bind(&*self.action_button, "icon-name", Some(obj));

            action_state_expr
                .chain_closure::<String>(closure!(|_: Self::Type, state: model::ActionState2| {
                    if state == model::ActionState2::Cancelled {
                        gettext("Abort")
                    } else {
                        gettext("Remove")
                    }
                }))
                .bind(&*self.action_button, "tooltip-text", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ActionRow {
        fn unroot(&self) {
            self.parent_unroot();

            let id = self.notification_id.get().unwrap().to_owned();

            rt::Promise::new(async move {
                match ashpd::notification::NotificationProxy::new().await {
                    Ok(proxy) => {
                        if let Err(e) = proxy.remove_notification(&id).await {
                            log::warn!("Failed to remove desktop notification: {e}");
                        }
                    }
                    Err(e) => {
                        log::debug!("Desktop notification portal unavailable: {e}");
                    }
                }
            })
            .spawn();
        }
    }

    impl ActionRow {
        pub(super) fn set_action(&self, value: Option<&model::BaseAction>) {
            let obj = &*self.obj();
            if obj.action().as_ref() == value {
                return;
            }

            if let Some(handler) = self.handler.take() {
                obj.action().unwrap().disconnect(handler);
            }

            if let Some(timer) = self.timer.take() {
                timer.remove();
            }

            if let Some(action) = value {
                obj.set_state_label(action);

                let handler = action.connect_state_notify(clone!(
                    #[weak]
                    obj,
                    move |action| {
                        obj.set_state_label(action);

                        if !matches!(
                            action.state(),
                            model::ActionState2::Failed | model::ActionState2::Finished
                        ) {
                            return;
                        }

                        let id = obj.imp().notification_id.get().unwrap().to_owned();
                        let notification = if action.state() == model::ActionState2::Failed {
                            ashpd::notification::Notification::new(&gettext("Failed Pods Action"))
                                .icon(ashpd::Icon::Names(vec![
                                    "computer-fail-symbolic".to_string(),
                                ]))
                                .priority(ashpd::notification::Priority::High)
                        } else {
                            ashpd::notification::Notification::new(&gettext("Finished Pods Action"))
                                .icon(ashpd::Icon::Names(vec![
                                    "checkbox-checked-symbolic".to_string(),
                                ]))
                                .priority(ashpd::notification::Priority::Low)
                        }
                        .body(action_description(action).as_str())
                        .default_action("");

                        rt::Promise::new(async move {
                            match ashpd::notification::NotificationProxy::new().await {
                                Ok(proxy) => {
                                    if let Err(e) = proxy.add_notification(&id, notification).await
                                    {
                                        log::warn!("Failed to send desktop notification: {e}");
                                    }
                                }
                                Err(e) => {
                                    log::debug!("Desktop notification portal unavailable: {e}");
                                }
                            }
                        })
                        .spawn();
                    }
                ));
                self.handler.replace(Some(handler));

                let timer = glib::timeout_add_seconds_local(
                    1,
                    clone!(
                        #[weak]
                        obj,
                        #[weak]
                        action,
                        #[upgrade_or]
                        glib::ControlFlow::Break,
                        move || {
                            let control_flow = obj.set_state_label(&action);
                            if control_flow.is_break()
                                && let Some(timer) = obj.imp().timer.take()
                            {
                                timer.remove();
                            }

                            control_flow
                        }
                    ),
                );
                self.timer.replace(Some(timer));
            }

            self.action.set(value);
            obj.notify_action();
        }
    }
}

glib::wrapper! {
    pub(crate) struct ActionRow(ObjectSubclass<imp::ActionRow>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ActionRow {
    fn set_state_label(&self, action: &model::BaseAction) -> glib::ControlFlow {
        let state_label = &*self.imp().state_label;

        match action.state() {
            model::ActionState2::Ongoing => {
                state_label.set_text(&gettext!(
                    "Ongoing ({})",
                    &utils::human_friendly_duration(
                        glib::DateTime::now_local().unwrap().to_unix() - action.start_timestamp()
                    )
                ));

                glib::ControlFlow::Continue
            }
            _ => {
                let duration = utils::human_friendly_duration(
                    action.end_timestamp() - action.start_timestamp(),
                );

                state_label.set_text(&match action.state() {
                    model::ActionState2::Cancelled => gettext!("Cancelled after {}", duration),
                    model::ActionState2::Failed => gettext!("Failed after {}", duration),
                    model::ActionState2::Finished => gettext!("Finished after {}", duration),
                    _ => unreachable!(),
                });

                glib::ControlFlow::Break
            }
        }
    }

    pub(crate) fn cancel_or_delete(&self) {
        let Some(action) = self.action() else {
            return;
        };

        if action.state() == model::ActionState2::Ongoing {
            action.cancel();
        } else if let Some(action_list) = action.action_list() {
            action_list.remove(&action);
        }
    }
}

fn action_image(action: &model::BaseAction) -> &str {
    if action
        .downcast_ref::<model::ContainerCreateAction>()
        .is_some()
    {
        "package-x-generic-symbolic"
    } else if action.downcast_ref::<model::ImagePullAction>().is_some() {
        "folder-download-symbolic"
    } else if action
        .downcast_ref::<model::ContainersPruneAction>()
        .is_some()
        || action.downcast_ref::<model::PodsPruneAction>().is_some()
        || action.downcast_ref::<model::ImagesPruneAction>().is_some()
        || action.downcast_ref::<model::VolumesPruneAction>().is_some()
    {
        "eraser5-symbolic"
    } else {
        ""
    }
}

fn action_description(action: &model::BaseAction) -> String {
    if let Some(action) = action.downcast_ref::<model::ContainerCreateAction>() {
        gettext!("Create <b>{}</b>", action.opts().name)
    } else if action
        .downcast_ref::<model::ContainersPruneAction>()
        .is_some()
    {
        gettext("Prune Containers")
    } else if action.downcast_ref::<model::PodsPruneAction>().is_some() {
        gettext("Prune Pods")
    } else if let Some(action) = action.downcast_ref::<model::ImagePullAction>() {
        gettext!("Pull <b>{}</b>", action.opts().reference)
    } else if action.downcast_ref::<model::ImagesPruneAction>().is_some() {
        gettext("Prune Images")
    } else if action.downcast_ref::<model::VolumesPruneAction>().is_some() {
        gettext("Prune Volumes")
    } else {
        gettext("Unknown Action")
    }
}
