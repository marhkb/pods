use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_CLEAN_UP: &str = "actions-menu-button.clean-up";
const PROGRESS_BAR_WIDTH: f64 = 24.0;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/actions/menu-button.ui")]
    pub(crate) struct MenuButton {
        pub(super) action_list: WeakRef<model::ActionList>,
        pub(super) overview: view::ActionsOverview,
        #[template_child]
        pub(super) progress_bar: TemplateChild<gtk::Fixed>,
        #[template_child]
        pub(super) progress_bar_through: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MenuButton {
        const NAME: &'static str = "PdsActionsMenuButton";
        type Type = super::MenuButton;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.set_css_name("actionsmenubutton");

            klass.install_action(ACTION_CLEAN_UP, None, move |widget, _, _| {
                widget.clean_up();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MenuButton {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "action-list",
                    "Action List",
                    "The action list of this menu button",
                    model::ActionList::static_type(),
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
                "action-list" => obj.set_action_list(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "action-list" => obj.action_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let popover_menu = self
                .menu_button
                .popover()
                .unwrap()
                .downcast::<gtk::PopoverMenu>()
                .unwrap();
            popover_menu.set_widget_name("actions-menu");
            popover_menu.add_child(&self.overview, "overview");

            let action_list_expr = Self::Type::this_expression("action-list");
            let len_expr = action_list_expr.chain_property::<model::ActionList>("len");
            let ongoing_expr = action_list_expr.chain_property::<model::ActionList>("ongoing");

            action_list_expr.bind(&self.overview, "action-list", Some(obj));

            gtk::ClosureExpression::new::<Vec<String>, _, _>(
                &[
                    action_list_expr.chain_property::<model::ActionList>("failed"),
                    action_list_expr.chain_property::<model::ActionList>("cancelled"),
                    action_list_expr.chain_property::<model::ActionList>("ongoing"),
                ],
                closure!(|_: Self::Type, failed: u32, cancelled: u32, ongoing: u32| {
                    vec![if failed > 0 {
                        "failed"
                    } else if cancelled > 0 {
                        "cancelled"
                    } else if ongoing > 0 {
                        "good"
                    } else {
                        "finished"
                    }
                    .to_string()]
                }),
            )
            .bind(obj, "css-classes", Some(obj));

            ongoing_expr
                .chain_closure::<i32>(closure!(|_: Self::Type, ongoing: u32| {
                    PROGRESS_BAR_WIDTH as i32 / (ongoing + 1) as i32
                }))
                .bind(&*self.progress_bar_through, "width-request", Some(obj));

            len_expr.watch(
                Some(obj),
                clone!(@weak obj => move || {
                    obj.action_set_enabled(
                        ACTION_CLEAN_UP,
                        obj.action_list().map(|list| list.len() > 0).unwrap_or(false)
                    );
                }),
            );

            len_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, len: u32| len > 0))
                .bind(&*self.progress_bar, "visible", Some(obj));
        }

        fn dispose(&self, obj: &Self::Type) {
            utils::ChildIter::from(obj).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for MenuButton {}
}

glib::wrapper! {
    pub(crate) struct MenuButton(ObjectSubclass<imp::MenuButton>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl MenuButton {
    pub(crate) fn action_list(&self) -> Option<model::ActionList> {
        self.imp().action_list.upgrade()
    }

    pub(crate) fn set_action_list(&self, value: Option<&model::ActionList>) {
        if self.action_list().as_ref() == value {
            return;
        }
        self.imp().action_list.set(value);
        self.notify("action-list");
    }

    fn clean_up(&self) {
        if let Some(action_list) = self.action_list() {
            action_list.clean_up();
        }
    }
}
