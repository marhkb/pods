use std::cell::OnceCell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use ashpd::desktop as ashpd;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

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
        pub(super) action: glib::WeakRef<model::Action>,
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
            let type_expr = action_expr.chain_property::<model::Action>("action-type");
            let description_expr = action_expr.chain_property::<model::Action>("description");
            let state_expr = action_expr.chain_property::<model::Action>("state");

            type_expr
                .chain_closure::<String>(closure!(|_: Self::Type, type_: model::ActionType| {
                    use model::ActionType::*;

                    match type_ {
                        PruneContainers | PruneImages | PrunePods | PruneVolumes => {
                            "eraser5-symbolic"
                        }
                        DownloadImage => "folder-download-symbolic",
                        BuildImage => "build-configure-symbolic",
                        PushImage => "put-symbolic",
                        Commit => "merge-symbolic",
                        CreateAndRunContainer => "media-playback-start-symbolic",
                        CreateContainer | Pod => "list-add-symbolic",
                        CopyFiles => "edit-copy-symbolic",
                        _ => unreachable!(),
                    }
                }))
                .bind(&*self.type_image, "icon-name", Some(obj));

            description_expr.bind(obj, "tooltip-markup", Some(obj));
            description_expr.bind(&*self.description_label, "label", Some(obj));

            let classes = utils::css_classes(self.state_label.upcast_ref());
            state_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, state: model::ActionState| {
                        use model::ActionState::*;

                        classes
                            .iter()
                            .cloned()
                            .chain(Some(String::from(match state {
                                Ongoing => "accent",
                                Finished => "dim-label",
                                Aborted => "warning",
                                Failed => "error",
                            })))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.state_label, "css-classes", Some(obj));

            state_expr
                .chain_closure::<String>(closure!(|_: Self::Type, state: model::ActionState| {
                    if state == model::ActionState::Ongoing {
                        "window-close-symbolic"
                    } else {
                        "cross-symbolic"
                    }
                }))
                .bind(&*self.action_button, "icon-name", Some(obj));

            state_expr
                .chain_closure::<String>(closure!(|_: Self::Type, state: model::ActionState| {
                    if state == model::ActionState::Ongoing {
                        gettext("Abort")
                    } else {
                        gettext("Remove")
                    }
                }))
                .bind(&*self.action_button, "tooltip-text", Some(obj));

            action_expr
                .chain_property::<model::Action>("num")
                .chain_closure::<Option<glib::Variant>>(closure!(|_: Self::Type, num: u32| {
                    Some(num.to_variant())
                }))
                .bind(&*self.action_button, "action-target", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ActionRow {
        fn unroot(&self) {
            self.parent_unroot();

            let id = self.notification_id.get().unwrap().to_owned();

            crate::runtime().spawn(async move {
                let _ = ashpd::notification::NotificationProxy::new()
                    .await
                    .unwrap()
                    .remove_notification(&id)
                    .await;
            });
        }
    }

    impl ActionRow {
        pub(super) fn set_action(&self, value: Option<&model::Action>) {
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

                let handler = action.connect_notify_local(
                    Some("state"),
                    clone!(
                        #[weak]
                        obj,
                        move |action, _| {
                            obj.set_state_label(action);

                            if !matches!(
                                action.state(),
                                model::ActionState::Failed | model::ActionState::Finished
                            ) {
                                return;
                            }

                            let id = obj.imp().notification_id.get().unwrap().to_owned();
                            let notification = if action.state() == model::ActionState::Failed {
                                ashpd::notification::Notification::new(&gettext(
                                    "Failed Pods Action",
                                ))
                                .icon(ashpd::Icon::Names(vec![
                                    "computer-fail-symbolic".to_string()
                                ]))
                                .priority(ashpd::notification::Priority::High)
                            } else {
                                ashpd::notification::Notification::new(&gettext(
                                    "Finished Pods Action",
                                ))
                                .icon(ashpd::Icon::Names(vec![
                                    "checkbox-checked-symbolic".to_string()
                                ]))
                                .priority(ashpd::notification::Priority::Low)
                            }
                            .body(action.description().as_ref())
                            .default_action("");

                            crate::runtime().spawn(async move {
                                let _ = ashpd::notification::NotificationProxy::new()
                                    .await
                                    .unwrap()
                                    .add_notification(&id, notification)
                                    .await;
                            });
                        }
                    ),
                );
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
                            if control_flow.is_break() {
                                if let Some(timer) = obj.imp().timer.take() {
                                    timer.remove();
                                }
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
    fn set_state_label(&self, action: &model::Action) -> glib::ControlFlow {
        let state_label = &*self.imp().state_label;

        match action.state() {
            model::ActionState::Ongoing => {
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
                    model::ActionState::Finished => gettext!("Finished after {}", duration),
                    model::ActionState::Aborted => gettext!("Aborted after {}", duration),
                    model::ActionState::Failed => gettext!("Failed after {}", duration),
                    _ => unreachable!(),
                });

                glib::ControlFlow::Break
            }
        }
    }
}
