use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use glib::closure;
use glib::subclass::InitializingObject;
use glib::subclass::Signal;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

const ACTIONS_OVERVIEW_ACTION_CLEAR_ACTIONS: &str = "actions-overview.clear-actions";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ActionsSidebar)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/actions_sidebar.ui")]
    pub(crate) struct ActionsSidebar {
        #[property(get, set = Self::set_action_list, nullable)]
        pub(super) action_list: glib::WeakRef<model::ActionList>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) action_list_view: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ActionsSidebar {
        const NAME: &'static str = "PdsActionsSidebar";
        type Type = super::ActionsSidebar;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.set_css_name("actionssidebar");

            klass.install_action(
                ACTIONS_OVERVIEW_ACTION_CLEAR_ACTIONS,
                None,
                |widget, _, _| {
                    widget.clear_actions();
                },
            );
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ActionsSidebar {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("cleared").build()])
        }

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

            Self::Type::this_expression("action-list")
                .chain_property::<model::ActionList>("len")
                .chain_closure::<String>(closure!(|_: Self::Type, len: u32| {
                    if len > 0 { "actions" } else { "empty" }
                }))
                .bind(&*self.stack, "visible-child-name", Some(obj));

            let action_list_expr = Self::Type::this_expression("action-list");
            let action_list_can_clear_expr = gtk::ClosureExpression::new::<bool>(
                [
                    action_list_expr.chain_property::<model::ActionList>("len"),
                    action_list_expr.chain_property::<model::ActionList>("ongoing"),
                ],
                closure!(|_: Self::Type, len: u32, ongoing: u32| len - ongoing > 0),
            );

            action_list_can_clear_expr.clone().watch(
                Some(obj),
                clone!(
                    #[weak]
                    obj,
                    move || {
                        obj.action_set_enabled(
                            ACTIONS_OVERVIEW_ACTION_CLEAR_ACTIONS,
                            action_list_can_clear_expr.evaluate_as(Some(&obj)).unwrap(),
                        );
                    }
                ),
            );
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ActionsSidebar {}

    #[gtk::template_callbacks]
    impl ActionsSidebar {
        #[template_callback]
        fn activated(&self, pos: u32) {
            let action = self
                .action_list_view
                .model()
                .unwrap()
                .item(pos)
                .unwrap()
                .downcast::<model::Action>()
                .unwrap();

            utils::Dialog::new(&*self.obj(), &view::ActionPage::from(&action)).present();
        }

        pub(super) fn set_action_list(&self, value: Option<&model::ActionList>) {
            let obj = self.obj();
            if obj.action_list().as_ref() == value {
                return;
            }

            if let Some(action_list) = value {
                let sorter = gtk::NumericSorter::builder()
                    .expression(model::Action::this_expression("start-timestamp"))
                    .sort_order(gtk::SortType::Descending)
                    .build();
                let model = gtk::SortListModel::new(Some(action_list.to_owned()), Some(sorter));

                self.action_list_view
                    .set_model(Some(&gtk::NoSelection::new(Some(model))));
            }

            self.action_list.set(value);
            obj.notify_action_list();
        }
    }
}

glib::wrapper! {
    pub(crate) struct ActionsSidebar(ObjectSubclass<imp::ActionsSidebar>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ActionsSidebar {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl ActionsSidebar {
    pub(crate) fn clear_actions(&self) {
        if let Some(action_list) = self.action_list() {
            action_list.clear();

            self.emit_by_name::<()>("cleared", &[]);
        }
    }
}
