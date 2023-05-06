use std::borrow::Cow;

use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::OnceCell as SyncOnceCell;

use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::Statusbar)]
    #[template(resource = "/com/github/marhkb/Pods/ui/statusbar.ui")]
    pub(crate) struct Statusbar {
        pub(super) css_provider: gtk::CssProvider,
        pub(super) connection_switcher: view::ConnectionSwitcher,
        pub(super) actions_sidebar: view::ActionsSidebar,
        #[property(get, set, nullable)]
        pub(super) connection_manager: glib::WeakRef<model::ConnectionManager>,
        #[template_child]
        pub(super) statusbar: TemplateChild<panel::Statusbar>,
        #[template_child]
        pub(super) connections_toggle_button: TemplateChild<gtk::ToggleButton>,
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
        pub(super) actions_toggle_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) actions_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) actions_label_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) actions_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Statusbar {
        const NAME: &'static str = "PdsStatusbar";
        type Type = super::Statusbar;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Statusbar {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: SyncOnceCell<Vec<glib::ParamSpec>> = SyncOnceCell::new();
            PROPERTIES.get_or_init(|| {
                Self::derived_properties()
                    .iter()
                    .cloned()
                    .chain(vec![
                        glib::ParamSpecBoolean::builder("show-connections-sidebar")
                            .explicit_notify()
                            .build(),
                        glib::ParamSpecBoolean::builder("show-actions-overview")
                            .explicit_notify()
                            .build(),
                    ])
                    .collect::<Vec<_>>()
            })
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "show-connections-sidebar" => {
                    self.obj()
                        .set_show_connections_sidebar(value.get().unwrap_or_default());
                }
                "show-actions-overview" => {
                    self.obj()
                        .set_show_actions_overview(value.get().unwrap_or_default());
                }
                _ => self.derived_set_property(id, value, pspec),
            }
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "show-connections-sidebar" => self.obj().is_show_connections_sidebar().to_value(),
                "show-actions-overview" => self.obj().is_show_actions_overview().to_value(),
                _ => self.derived_property(id, pspec),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            self.connections_toggle_button.connect_notify_local(
                Some("active"),
                clone!(@weak obj => move |_, _| {
                    if obj.is_show_connections_sidebar() {
                        obj.set_show_actions_sidebar(false);
                    }
                    obj.notify("show-connections-sidebar");
                }),
            );

            self.actions_toggle_button.connect_notify_local(
                Some("active"),
                clone!(@weak obj => move |_, _| {
                    if obj.is_show_actions_overview() {
                        obj.set_show_connection_switcher(false);
                    }
                    obj.notify("show-actions-overview");
                }),
            );

            self.statusbar
                .style_context()
                .add_provider(&self.css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

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
                &self.connection_switcher,
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
            .bind(&*self.actions_image, "icon-name", Some(obj));

            podman_version_expr
                .chain_closure::<String>(closure!(|_: Self::Type, version: Option<String>| version
                    .map(|_| "version")
                    .unwrap_or("loading")))
                .bind(&*self.podman_stack, "visible-child-name", Some(obj));
            podman_version_expr
                // .chain_closure::<String>(closure!(|_: Self::Type, version: Option<String>| version))
                .bind(&*self.podman_version_label, "label", Some(obj));

            action_list_expr.bind(&self.actions_sidebar, "action-list", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [&action_list_ongoing_expr, &action_list_len_expr],
                closure!(|_: Self::Type, ongoing: u32, len: u32| {
                    format!("{}/{len}", len - ongoing)
                }),
            )
            .bind(&*self.actions_label, "label", Some(obj));

            action_list_len_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, len: u32| len > 0))
                .bind(&*self.actions_label_revealer, "reveal-child", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
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
    pub(crate) fn is_show_connections_sidebar(&self) -> bool {
        self.imp().connections_toggle_button.is_active()
    }

    pub(crate) fn set_show_connections_sidebar(&self, value: bool) {
        self.imp().connections_toggle_button.set_active(value);
    }

    pub(crate) fn is_show_actions_overview(&self) -> bool {
        self.imp().actions_toggle_button.is_active()
    }

    pub(crate) fn set_show_actions_overview(&self, value: bool) {
        self.imp().actions_toggle_button.set_active(value);
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

        self.imp().css_provider.load_from_data(&format!(
            "panelstatusbar {{ background: shade({bg_color}, 1.2); color: {fg_color}; }}"
        ));
    }
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
