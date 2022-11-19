use adw::traits::ActionRowExt;
use gettextrs::gettext;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;
use sourceview5::traits::BufferExt;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_COPY_SOCKET_ACTIVATION_COMMAND: &str =
    "connection-creator-page.copy-socket-activation-command";
const ACTION_COPY_ROOT_SYSTEMD_UNIT_PATH: &str =
    "connection-creator-page.copy-root-systemd-unit-path";
const ACTION_COPY_ROOT_SYSTEMD_UNIT_CONTENT: &str =
    "connection-creator-page.copy-root-systemd-unit-content";
const ACTION_COPY_ROOT_SOCKET_ACTIVATION_COMMAND: &str =
    "connection-creator-page.copy-root-socket-activation-command";
const ACTION_COPY_ROOT_URL: &str = "connection-creator-page.copy-root-url";
const ACTION_TRY_CONNECT: &str = "connection-creator-page.try-connect";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/connection/creation-page.ui")]
    pub(crate) struct CreationPage {
        pub(super) connection_manager: OnceCell<model::ConnectionManager>,
        #[template_child]
        pub(super) back_navigation_controls: TemplateChild<view::BackNavigationControls>,
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
        pub(super) root_systemd_unit_path_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) root_systemd_unit_content_buffer: TemplateChild<sourceview5::Buffer>,
        #[template_child]
        pub(super) root_socket_activation_command_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) root_url_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) color_button: TemplateChild<gtk::ColorButton>,
        #[template_child]
        pub(super) color_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) connect_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CreationPage {
        const NAME: &'static str = "PdsConnectionCreationPage";
        type Type = super::CreationPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(
                ACTION_COPY_SOCKET_ACTIVATION_COMMAND,
                None,
                move |widget, _, _| {
                    widget.copy_socket_acivation_command();
                },
            );
            klass.install_action(
                ACTION_COPY_ROOT_SYSTEMD_UNIT_PATH,
                None,
                move |widget, _, _| {
                    widget.copy_root_systemd_unit_path();
                },
            );
            klass.install_action(
                ACTION_COPY_ROOT_SYSTEMD_UNIT_CONTENT,
                None,
                move |widget, _, _| {
                    widget.copy_root_systemd_unit_content();
                },
            );
            klass.install_action(
                ACTION_COPY_ROOT_SOCKET_ACTIVATION_COMMAND,
                None,
                move |widget, _, _| {
                    widget.copy_root_socket_acivation_command();
                },
            );
            klass.install_action(ACTION_COPY_ROOT_URL, None, move |widget, _, _| {
                widget.copy_root_url();
            });
            klass.install_action(ACTION_TRY_CONNECT, None, move |widget, _, _| {
                widget.try_connect();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CreationPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::ConnectionManager>(
                    "connection-manager",
                )
                .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "connection-manager" => self.connection_manager.set(value.get().unwrap()).unwrap(),
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

            obj.action_set_enabled(ACTION_TRY_CONNECT, !self.name_entry_row.text().is_empty());
            self.name_entry_row
                .connect_changed(clone!(@weak obj => move |entry| {
                    obj.action_set_enabled(ACTION_TRY_CONNECT, !entry.text().is_empty())
                }));

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

            let adw_style_manager = adw::StyleManager::default();
            obj.on_notify_dark(&adw_style_manager);
            adw_style_manager.connect_dark_notify(clone!(@weak obj => move |style_manager| {
                obj.on_notify_dark(style_manager);
            }));
            self.root_systemd_unit_content_buffer.set_language(
                sourceview5::LanguageManager::default()
                    .language("ini")
                    .as_ref(),
            );

            self.color_button
                .set_rgba(&gdk::RGBA::new(0.207, 0.517, 0.894, 1.0));
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
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
            utils::root(widget).set_default_widget(Some(&*self.connect_button));
        }

        fn unroot(&self) {
            utils::root(&*self.obj()).set_default_widget(gtk::Widget::NONE);
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
        glib::Object::builder::<Self>()
            .property("connection-manager", connection_manager)
            .build()
    }
}

impl CreationPage {
    pub(crate) fn connection_manager(&self) -> &model::ConnectionManager {
        self.imp().connection_manager.get().unwrap()
    }

    fn copy_socket_acivation_command(&self) {
        let label = &*self.imp().socket_activation_command_label;
        label.select_region(0, -1);
        label.emit_copy_clipboard();
    }

    fn copy_root_systemd_unit_path(&self) {
        let label = &*self.imp().root_systemd_unit_path_label;
        label.select_region(0, -1);
        label.emit_copy_clipboard();
    }

    fn copy_root_systemd_unit_content(&self) {
        let buffer = &*self.imp().root_systemd_unit_content_buffer;
        buffer.select_range(&buffer.start_iter(), &buffer.end_iter());
        buffer.copy_clipboard(&gdk::Display::default().unwrap().clipboard());
    }

    fn copy_root_socket_acivation_command(&self) {
        let label = &*self.imp().root_socket_activation_command_label;
        label.select_region(0, -1);
        label.emit_copy_clipboard();
    }

    fn copy_root_url(&self) {
        let label = &*self.imp().root_url_label;
        label.select_region(0, -1);
        label.emit_copy_clipboard();
    }

    fn try_connect(&self) {
        if view::show_ongoing_actions_warning_dialog(
            self,
            self.connection_manager(),
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
                    Ok(_) => obj.imp().back_navigation_controls.navigate_to_first(),
                    Err(e) => obj.on_error(e),
                }),
            ) {
                self.on_error(e);
            }
        }
    }

    fn on_notify_dark(&self, style_manager: &adw::StyleManager) {
        self.imp()
            .root_systemd_unit_content_buffer
            .set_style_scheme(
                sourceview5::StyleSchemeManager::default()
                    .scheme(if style_manager.is_dark() {
                        "Adwaita-dark"
                    } else {
                        "Adwaita"
                    })
                    .as_ref(),
            );
    }

    fn on_error(&self, e: impl ToString) {
        utils::show_error_toast(self, &gettext("Error"), &e.to_string());
    }
}
