use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_CLEAN_UP_ACTIONS: &str = "statusbar.clean-up-actions";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/statusbar.ui")]
    pub(crate) struct Statusbar {
        pub(super) connection_manager: glib::WeakRef<model::ConnectionManager>,
        pub(super) connection_switcher_widget: view::ConnectionSwitcherWidget,
        pub(super) actions_overview: view::ActionsOverview,
        #[template_child]
        pub(super) connections_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) connection_image_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) active_connection_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) active_connection_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) notifications_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) notifications_progress_bar_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) notifications_progress_bar: TemplateChild<gtk::ProgressBar>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Statusbar {
        const NAME: &'static str = "PdsStatusbar";
        type Type = super::Statusbar;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_CLEAN_UP_ACTIONS, None, move |widget, _, _| {
                widget.clean_up_actions();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Statusbar {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::ConnectionManager>(
                    "connection-manager",
                )
                .explicit_notify()
                .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "connection-manager" => self.obj().set_connection_manager(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "connection-manager" => self.obj().connection_manager().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            add_menu_items(
                &self.connections_menu_button,
                self.connection_switcher_widget.upcast_ref(),
            );
            add_menu_items(
                &self.notifications_menu_button,
                self.actions_overview.upcast_ref(),
            );

            let connection_manager_expr = Self::Type::this_expression("connection-manager");
            let client_expr =
                connection_manager_expr.chain_property::<model::ConnectionManager>("client");
            let connection_expr = client_expr.chain_property::<model::Client>("connection");

            let action_list_expr = client_expr.chain_property::<model::Client>("action-list");
            let action_list_ongoing_expr =
                action_list_expr.chain_property::<model::ActionList>("ongoing");
            let action_list_len_expr = action_list_expr.chain_property::<model::ActionList>("len");

            connection_manager_expr.bind(
                &self.connection_switcher_widget,
                "connection-manager",
                Some(obj),
            );

            connection_manager_expr
                .chain_property::<model::ConnectionManager>("connecting")
                .chain_closure::<String>(closure!(
                    |_: Self::Type, connecting: bool| if connecting {
                        "connecting"
                    } else {
                        "image"
                    }
                ))
                .bind(
                    &*self.connection_image_stack,
                    "visible-child-name",
                    Some(obj),
                );

            connection_expr
                .chain_property::<model::Connection>("is-remote")
                .chain_closure::<String>(closure!(|_: Self::Type, is_remote: bool| {
                    if is_remote {
                        "network-server-symbolic"
                    } else {
                        "computer-symbolic"
                    }
                }))
                .bind(&*self.active_connection_image, "icon-name", Some(obj));
            connection_expr
                .chain_property::<model::Connection>("name")
                .bind(&*self.active_connection_label, "label", Some(obj));

            let css_classes = self.notifications_progress_bar.css_classes();
            gtk::ClosureExpression::new::<Vec<String>>(
                &[
                    action_list_expr.chain_property::<model::ActionList>("failed"),
                    action_list_expr.chain_property::<model::ActionList>("cancelled"),
                    action_list_expr.chain_property::<model::ActionList>("ongoing"),
                ],
                closure!(|_: Self::Type, failed: u32, cancelled: u32, ongoing: u32| {
                    css_classes
                        .iter()
                        .cloned()
                        .chain(
                            if failed > 0 {
                                Some("error")
                            } else if cancelled > 0 {
                                Some("warning")
                            } else if ongoing == 0 {
                                Some("success")
                            } else {
                                None
                            }
                            .map(glib::GString::from),
                        )
                        .collect::<Vec<_>>()
                }),
            )
            .bind(&*self.notifications_progress_bar, "css-classes", Some(obj));

            action_list_expr.bind(&self.actions_overview, "action-list", Some(obj));
            action_list_ongoing_expr
                .chain_closure::<f64>(closure!(|_: Self::Type, ongoing: u32| {
                    1.0 / (ongoing + 1) as f64
                }))
                .bind(&*self.notifications_progress_bar, "fraction", Some(obj));
            action_list_len_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, len: u32| len > 0))
                .bind(
                    &*self.notifications_progress_bar_revealer,
                    "reveal-child",
                    Some(obj),
                );
            action_list_len_expr.watch(
                Some(obj),
                clone!(@weak obj => move || {
                    obj.action_set_enabled(
                        ACTION_CLEAN_UP_ACTIONS,
                        obj.action_list().map(|list| list.len() > 0).unwrap_or(false)
                    );
                }),
            );
        }

        fn dispose(&self) {
            utils::ChildIter::from(self.obj().upcast_ref()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for Statusbar {}
}

glib::wrapper! {
    pub(crate) struct Statusbar(ObjectSubclass<imp::Statusbar>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Statusbar {
    pub(crate) fn connection_manager(&self) -> Option<model::ConnectionManager> {
        self.imp().connection_manager.upgrade()
    }

    pub(crate) fn set_connection_manager(&self, value: Option<&model::ConnectionManager>) {
        if self.connection_manager().as_ref() == value {
            return;
        }
        self.imp().connection_manager.set(value);
        self.notify("connection-manager");
    }

    pub(crate) fn action_list(&self) -> Option<model::ActionList> {
        self.connection_manager()
            .as_ref()
            .and_then(model::ConnectionManager::client)
            .as_ref()
            .map(model::Client::action_list)
            .cloned()
    }

    fn clean_up_actions(&self) {
        if let Some(action_list) = self.action_list() {
            action_list.clean_up();
        }
    }
}

fn add_menu_items(menu_button: &gtk::MenuButton, widget: &gtk::Widget) {
    let popover_menu = menu_button
        .popover()
        .unwrap()
        .downcast::<gtk::PopoverMenu>()
        .unwrap();

    popover_menu.add_child(widget, "items");
}
