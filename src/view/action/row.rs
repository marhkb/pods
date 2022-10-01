use gettextrs::gettext;
use glib::subclass::InitializingObject;
use gtk::glib;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/action/row.ui")]
    pub(crate) struct Row {
        pub(super) action: WeakRef<model::Action>,
        #[template_child]
        pub(super) type_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) name_label: TemplateChild<gtk::Label>,
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
                vec![glib::ParamSpecObject::new(
                    "action",
                    "Action",
                    "The action of this row",
                    model::Action::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                )]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "action" => obj.set_action(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "action" => obj.action().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let action_expr = Self::Type::this_expression("action");
            let type_expr = action_expr.chain_property::<model::Action>("type");
            let name_expr = action_expr.chain_property::<model::Action>("name");
            let state_expr = action_expr.chain_property::<model::Action>("state");

            type_expr
                .chain_closure::<String>(closure!(|_: Self::Type, type_: model::ActionType| {
                    use model::ActionType::*;

                    match type_ {
                        PruneImages => "larger-brush-symbolic",
                        DownloadImage | BuildImage => "image-x-generic-symbolic",
                        Container => "package-x-generic-symbolic",
                        Pod => "pods-symbolic",
                        Undefined => unreachable!(),
                    }
                }))
                .bind(&*self.type_image, "icon-name", Some(obj));

            gtk::ClosureExpression::new::<String, _, _>(
                &[type_expr, name_expr],
                closure!(|_: Self::Type, type_: model::ActionType, name: String| {
                    use model::ActionType::*;

                    match type_ {
                        PruneImages => gettext("Prune Images"),
                        DownloadImage => gettext!("Download {}", name),
                        BuildImage => gettext!("Build {}", name),
                        Container | Pod => gettext!("Create {}", name),
                        Undefined => unreachable!(),
                    }
                }),
            )
            .bind(&*self.name_label, "label", Some(obj));

            state_expr
                .chain_closure::<String>(closure!(|_: Self::Type, state: model::ActionState| {
                    use model::ActionState::*;

                    match state {
                        Ongoing => gettext("Ongoing"),
                        Finished => gettext("Finished"),
                        Cancelled => gettext("Cancelled"),
                        Failed => gettext("Failed"),
                    }
                }))
                .bind(&*self.state_label, "label", Some(obj));

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

            action_expr
                .chain_property::<model::Action>("num")
                .chain_closure::<Option<glib::Variant>>(closure!(|_: Self::Type, num: u32| {
                    Some(num.to_variant())
                }))
                .bind(&*self.action_button, "action-target", Some(obj));
        }

        fn dispose(&self, obj: &Self::Type) {
            utils::ChildIter::from(obj).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for Row {}
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
        self.imp().action.set(value);
        self.notify("action");
    }
}
