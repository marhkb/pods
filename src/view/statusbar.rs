use std::borrow::Cow;

use gtk::gdk;
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
        pub(super) css_provider: gtk::CssProvider,
        pub(super) connection_manager: glib::WeakRef<model::ConnectionManager>,
        pub(super) connection_switcher_widget: view::ConnectionSwitcherWidget,
        pub(super) actions_overview: view::ActionsOverview,
        #[template_child]
        pub(super) statusbar: TemplateChild<panel::Statusbar>,
        #[template_child]
        pub(super) connections_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) connection_image_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) active_connection_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) active_connection_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) podman_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) podman_version_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) notifications_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) notifications_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) notifications_label_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) notifications_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Statusbar {
        const NAME: &'static str = "PdsStatusbar";
        type Type = super::Statusbar;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_CLEAN_UP_ACTIONS, None, |widget, _, _| {
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

            self.statusbar
                .style_context()
                .add_provider(&self.css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

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

            let podman_version_expr = client_expr.chain_property::<model::Client>("version");

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
                        "local-connection-symbolic"
                    }
                }))
                .bind(&*self.active_connection_image, "icon-name", Some(obj));
            connection_expr
                .chain_property::<model::Connection>("name")
                .bind(&*self.active_connection_label, "label", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [
                    &action_list_expr.chain_property::<model::ActionList>("failed"),
                    &action_list_expr.chain_property::<model::ActionList>("cancelled"),
                    &action_list_ongoing_expr,
                    &action_list_len_expr,
                ],
                closure!(
                    |_: Self::Type, failed: u32, cancelled: u32, ongoing: u32, len: u32| {
                        if failed > 0 {
                            "error"
                        } else if cancelled > 0 {
                            "dialog-warning-symbolic"
                        } else if ongoing == 0 && len > 0 {
                            "success"
                        } else {
                            "bell-symbolic"
                        }
                    }
                ),
            )
            .bind(&*self.notifications_image, "icon-name", Some(obj));

            podman_version_expr
                .chain_closure::<String>(closure!(|_: Self::Type, version: Option<String>| version
                    .map(|_| "version")
                    .unwrap_or("loading")))
                .bind(&*self.podman_stack, "visible-child-name", Some(obj));
            podman_version_expr
                // .chain_closure::<String>(closure!(|_: Self::Type, version: Option<String>| version))
                .bind(&*self.podman_version_label, "label", Some(obj));

            action_list_expr.bind(&self.actions_overview, "action-list", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [&action_list_ongoing_expr, &action_list_len_expr],
                closure!(|_: Self::Type, ongoing: u32, len: u32| {
                    format!("{}/{len}", len - ongoing)
                }),
            )
            .bind(&*self.notifications_label, "label", Some(obj));

            action_list_len_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, len: u32| len > 0))
                .bind(
                    &*self.notifications_label_revealer,
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

    pub(crate) fn set_background(&self, bg_color: Option<gdk::RGBA>) {
        let (bg_color, fg_color) = bg_color
            .map(|color| {
                (
                    Cow::Owned(color.to_string()),
                    if luminance(&color) > 0.2 {
                        "rgba(0, 0, 0, 0.8)"
                    } else {
                        "#ffffff"
                    },
                )
            })
            .unwrap_or_else(|| (Cow::Borrowed("@headerbar_bg_color"), "@headerbar_fg_color"));

        self.imp().css_provider.load_from_data(
            format!("panelstatusbar {{ background: shade({bg_color}, 1.2); color: {fg_color}; }}")
                .as_bytes(),
        );
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

fn srgb(c: f32) -> f32 {
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(color: &gdk::RGBA) -> f32 {
    let red = srgb(color.red());
    let blue = srgb(color.blue());
    let green = srgb(color.green());
    red * 0.2126 + blue * 0.0722 + green * 0.7152
}
