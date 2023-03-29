use adw::traits::ActionRowExt;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::unsync::OnceCell as UnsyncOnceCell;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_COPY_SOCKET_ACTIVATION_COMMAND: &str =
    "connection-creator-page.copy-socket-activation-command";
const ACTION_SHOW_CUSTOM_INFO_DIALOG: &str = "connection-creation-page.show-custom-info-dialog";
const ACTION_TRY_CONNECT: &str = "connection-creation-page.try-connect";

const ACTION_ABORT: &str = "connection-creation-page.abort";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::CreationPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/connection/creation-page.ui")]
    pub(crate) struct CreationPage {
        #[property(get, set, construct_only)]
        pub(super) connection_manager: UnsyncOnceCell<model::ConnectionManager>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) connect_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) preferences_page: TemplateChild<adw::PreferencesPage>,
        #[template_child]
        pub(super) name_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) unix_socket_url_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) socket_activation_command_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) socket_url_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) custom_url_radio_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) url_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) color_button: TemplateChild<gtk::ColorButton>,
        #[template_child]
        pub(super) color_switch: TemplateChild<gtk::Switch>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CreationPage {
        const NAME: &'static str = "PdsConnectionCreationPage";
        type Type = super::CreationPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(
                ACTION_COPY_SOCKET_ACTIVATION_COMMAND,
                None,
                move |widget, _, _| {
                    widget.copy_socket_acivation_command();
                },
            );
            klass.install_action(ACTION_SHOW_CUSTOM_INFO_DIALOG, None, |widget, _, _| {
                widget.show_custom_info_dialog();
            });
            klass.install_action(ACTION_TRY_CONNECT, None, |widget, _, _| {
                widget.try_connect();
            });
            klass.install_action(ACTION_ABORT, None, |widget, _, _| {
                widget.abort();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CreationPage {
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

            let connecting_expr = Self::Type::this_expression("connection-manager")
                .chain_property::<model::ConnectionManager>("connecting");

            connecting_expr
                .chain_closure::<String>(closure!(|_: Self::Type, connecting: bool| {
                    if connecting {
                        "abort"
                    } else {
                        "connect"
                    }
                }))
                .bind(&*self.stack, "visible-child-name", Some(obj));
            connecting_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, connecting: bool| !connecting))
                .bind(&*self.preferences_page, "sensitive", Some(obj));

            obj.update_actions();
            self.name_entry_row
                .connect_changed(clone!(@weak obj => move |_| obj.update_actions()));

            obj.connection_manager().connect_notify_local(
                Some("connecting"),
                clone!(@weak obj => move|_ ,_|  obj.update_actions()),
            );

            self.unix_socket_url_row
                .set_subtitle(&utils::unix_socket_url());

            self.socket_url_label.set_markup(&gettext!(
                // Translators: The placeholder '{}' is replaced by 'official documentation'.
                "Visit the {} for more information.",
                format!(
                    "<a href=\"https://github.com/containers/podman/blob/cea9340242f3f6cf41f20fb0b6239aa3db5decd6/docs/tutorials/socket_activation.md\">{}</a>",
                    gettext("official documentation")
                )
            ));

            self.custom_url_radio_button
                .set_active(obj.connection_manager().contains_local_connection());

            self.color_button
                .set_rgba(&gdk::RGBA::new(0.207, 0.517, 0.894, 1.0));
        }

        fn dispose(&self) {
            let obj = &*self.obj();

            obj.abort();
            utils::ChildIter::from(obj.upcast_ref()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for CreationPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            glib::idle_add_local(
                clone!(@weak widget => @default-return glib::Continue(false), move || {
                    widget.imp().name_entry_row.grab_focus();
                    glib::Continue(false)
                }),
            );
            utils::root(widget.upcast_ref()).set_default_widget(Some(&*self.connect_button));
        }

        fn unroot(&self) {
            utils::root(self.obj().upcast_ref()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }
}

glib::wrapper! {
    pub(crate) struct CreationPage(ObjectSubclass<imp::CreationPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::ConnectionManager> for CreationPage {
    fn from(connection_manager: &model::ConnectionManager) -> Self {
        glib::Object::builder()
            .property("connection-manager", connection_manager)
            .build()
    }
}

impl CreationPage {
    pub(crate) fn copy_socket_acivation_command(&self) {
        let label = &*self.imp().socket_activation_command_label;
        label.select_region(0, -1);
        label.emit_copy_clipboard();
    }

    pub(crate) fn show_custom_info_dialog(&self) {
        let dialog = view::ConnectionCustomInfoDialog::default();
        dialog.set_transient_for(Some(&utils::root(self.upcast_ref())));
        dialog.present();
    }

    pub(crate) fn try_connect(&self) {
        if view::show_ongoing_actions_warning_dialog(
            self.upcast_ref(),
            &self.connection_manager(),
            &gettext("Confirm Connecting to New Instance"),
        ) {
            let imp = self.imp();

            if let Err(e) = self.connection_manager().try_connect(
                imp.name_entry_row.text().as_str(),
                if imp.custom_url_radio_button.is_active() {
                    imp.url_entry_row.text().into()
                } else {
                    utils::unix_socket_url()
                }
                .as_ref(),
                if imp.color_switch.is_active() {
                    Some(imp.color_button.rgba())
                } else {
                    None
                },
                clone!(@weak self as obj => move |result| match result {
                    Ok(_) => obj.activate_action("action.cancel", None).unwrap(),
                    Err(e) => obj.on_error(&e.to_string()),
                }),
            ) {
                self.on_error(&e.to_string());
            }
        }
    }

    pub(crate) fn abort(&self) {
        self.connection_manager().abort();
    }

    fn update_actions(&self) {
        let is_connecting = self.connection_manager().is_connecting();

        self.action_set_enabled(
            ACTION_TRY_CONNECT,
            !is_connecting && !self.imp().name_entry_row.text().is_empty(),
        );
        self.action_set_enabled(ACTION_ABORT, is_connecting);
    }

    fn on_error(&self, msg: &str) {
        utils::show_error_toast(self.upcast_ref(), &gettext("Error"), msg);
    }
}
