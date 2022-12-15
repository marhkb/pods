use std::cell::RefCell;

use ashpd::desktop as ashpd;
use gettextrs::gettext;
use glib::subclass::InitializingObject;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/action/row.ui")]
    pub(crate) struct Row {
        pub(super) action: glib::WeakRef<model::Action>,
        pub(super) notification_id: OnceCell<glib::GString>,
        pub(super) handler: RefCell<Option<glib::SignalHandlerId>>,
        pub(super) timer: RefCell<Option<glib::SourceId>>,
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
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsActionRow";
        type Type = super::Row;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Row {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::Action>("action")
                    .explicit_notify()
                    .build()]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "action" => self.obj().set_action(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "action" => self.obj().action().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            self.notification_id
                .set(glib::uuid_string_random())
                .unwrap();

            let action_expr = Self::Type::this_expression("action");
            let type_expr = action_expr.chain_property::<model::Action>("type");
            let description_expr = action_expr.chain_property::<model::Action>("description");
            let state_expr = action_expr.chain_property::<model::Action>("state");

            type_expr
                .chain_closure::<String>(closure!(|_: Self::Type, type_: model::ActionType| {
                    use model::ActionType::*;

                    match type_ {
                        PruneImages => "larger-brush-symbolic",
                        DownloadImage => "folder-download-symbolic",
                        BuildImage => "build-configure-symbolic",
                        Commit => "merge-symbolic",
                        Container | Pod => "list-add-symbolic",
                        CopyFiles => "edit-copy-symbolic",
                        _ => unreachable!(),
                    }
                }))
                .bind(&*self.type_image, "icon-name", Some(obj));

            description_expr.bind(&*self.description_label, "label", Some(obj));

            let classes = self.state_label.css_classes();
            state_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, state: model::ActionState| {
                        use model::ActionState::*;

                        classes
                            .iter()
                            .cloned()
                            .chain(Some(glib::GString::from(match state {
                                Ongoing => "accent",
                                Finished => "success",
                                Cancelled => "warning",
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
                        "user-trash-symbolic"
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
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for Row {
        fn unroot(&self) {
            self.parent_unroot();

            let id = self.notification_id.get().unwrap().to_owned();

            glib::MainContext::default().spawn(async move {
                let _ = ashpd::notification::NotificationProxy::new()
                    .await
                    .unwrap()
                    .remove_notification(&id)
                    .await;
            });
        }
    }
}

glib::wrapper! {
    pub(crate) struct Row(ObjectSubclass<imp::Row>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Row {
    pub(crate) fn action(&self) -> Option<model::Action> {
        self.imp().action.upgrade()
    }

    pub(crate) fn set_action(&self, value: Option<&model::Action>) {
        if self.action().as_ref() == value {
            return;
        }

        let imp = self.imp();

        if let Some(handler) = imp.handler.take() {
            self.action().unwrap().disconnect(handler);
        }

        if let Some(timer) = imp.timer.take() {
            timer.remove();
        }

        if let Some(action) = value {
            self.set_state_label(action);

            let handler = action.connect_notify_local(
                Some("state"),
                clone!(@weak self as obj => move |action, _| {
                    obj.set_state_label(action);

                    if !matches!(action.state(), model::ActionState::Failed | model::ActionState::Finished) {
                        return;
                    }

                    let id = obj.imp().notification_id.get().unwrap().to_owned();
                    let notification = if action.state() == model::ActionState::Failed {
                        ashpd::notification::Notification::new(&gettext("Failed Pods Action"))
                            .icon(ashpd::Icon::Names(vec!["computer-fail-symbolic".to_string()]))
                            .priority(ashpd::notification::Priority::High)
                    } else {
                        ashpd::notification::Notification::new(&gettext("Finished Pods Action"))
                            .icon(ashpd::Icon::Names(vec!["checkbox-checked-symbolic".to_string()]))
                            .priority(ashpd::notification::Priority::Low)
                    }
                    .body(action.description())
                    .default_action("");

                    glib::MainContext::default().spawn_local(async move {
                        let _ = ashpd::notification::NotificationProxy::new().await.unwrap()
                            .add_notification(&id, notification)
                            .await;
                    });
                }),
            );
            imp.handler.replace(Some(handler));

            let timer = glib::timeout_add_seconds_local(
                1,
                clone!(@weak self as obj, @weak action => @default-return glib::Continue(false), move || {
                    let is_ongoing = obj.set_state_label(&action);
                    if !is_ongoing {
                        if let Some(timer) = obj.imp().timer.take() {
                            timer.remove();
                        }
                    }
                    glib::Continue(is_ongoing)
                }),
            );
            imp.timer.replace(Some(timer));
        }

        imp.action.set(value);
        self.notify("action");
    }

    fn set_state_label(&self, action: &model::Action) -> bool {
        let state_label = &*self.imp().state_label;

        match action.state() {
            model::ActionState::Ongoing => {
                state_label.set_text(&gettext!(
                    "Ongoing ({})",
                    &utils::human_friendly_duration(
                        glib::DateTime::now_local().unwrap().to_unix() - action.start_timestamp()
                    )
                ));

                true
            }
            _ => {
                let duration = utils::human_friendly_duration(
                    action.end_timestamp() - action.start_timestamp(),
                );

                state_label.set_text(&match action.state() {
                    model::ActionState::Finished => gettext!("Finished after {}", duration),
                    model::ActionState::Cancelled => gettext!("Cancelled after {}", duration),
                    model::ActionState::Failed => gettext!("Failed after {}", duration),
                    _ => unreachable!(),
                });

                false
            }
        }
    }
}
