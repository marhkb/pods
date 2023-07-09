use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use gtk::gdk;
use gtk::glib;
use gtk::CompositeTemplate;
use sourceview5::prelude::*;

const ACTION_COPY_ROOT_SYSTEMD_UNIT_PATH: &str =
    "connection-custom-info-dialog.copy-root-systemd-unit-path";
const ACTION_COPY_ROOT_SYSTEMD_UNIT_CONTENT: &str =
    "connection-custom-info-dialog.copy-root-systemd-unit-content";
const ACTION_COPY_ROOT_SOCKET_ACTIVATION_COMMAND: &str =
    "connection-custom-info-dialog.copy-root-socket-activation-command";
const ACTION_COPY_ROOT_URL: &str = "connection-custom-info-dialog.copy-root-url";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/connection_custom_info_dialog.ui")]
    pub(crate) struct ConnectionCustomInfoDialog {
        #[template_child]
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) root_systemd_unit_path_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) root_systemd_unit_content_buffer: TemplateChild<sourceview5::Buffer>,
        #[template_child]
        pub(super) root_socket_activation_command_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) root_url_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConnectionCustomInfoDialog {
        const NAME: &'static str = "PdsConnectionCustomInfoDialog";
        type Type = super::ConnectionCustomInfoDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("win.close", None, |widget, _, _| {
                widget.close();
            });
            klass.add_binding_action(
                gdk::Key::W,
                gdk::ModifierType::CONTROL_MASK,
                "win.close",
                None,
            );
            klass.add_binding_action(
                gdk::Key::Escape,
                gdk::ModifierType::empty(),
                "win.close",
                None,
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
            klass.install_action(ACTION_COPY_ROOT_URL, None, |widget, _, _| {
                widget.copy_root_url();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ConnectionCustomInfoDialog {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            self.root_systemd_unit_content_buffer.set_language(
                sourceview5::LanguageManager::default()
                    .language("ini")
                    .as_ref(),
            );

            let style_manager = adw::StyleManager::default();
            style_manager.connect_dark_notify(clone!(@weak obj => move |style_manager| {
                obj.imp().on_notify_dark(style_manager);
            }));
            self.on_notify_dark(&style_manager);
        }
    }

    impl WidgetImpl for ConnectionCustomInfoDialog {}
    impl WindowImpl for ConnectionCustomInfoDialog {}
    impl AdwWindowImpl for ConnectionCustomInfoDialog {}

    impl ConnectionCustomInfoDialog {
        fn on_notify_dark(&self, style_manager: &adw::StyleManager) {
            self.root_systemd_unit_content_buffer.set_style_scheme(
                sourceview5::StyleSchemeManager::default()
                    .scheme(if style_manager.is_dark() {
                        "solarized-dark"
                    } else {
                        "solarized-light"
                    })
                    .as_ref(),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct ConnectionCustomInfoDialog(ObjectSubclass<imp::ConnectionCustomInfoDialog>)
    @extends gtk::Widget, gtk::Window, adw::Window,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for ConnectionCustomInfoDialog {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl ConnectionCustomInfoDialog {
    fn copy_root_systemd_unit_path(&self) {
        let label = &*self.imp().root_systemd_unit_path_label;
        label.select_region(0, -1);
        label.emit_copy_clipboard();

        self.show_toast(&gettext("systemd unit path copied"));
    }

    fn copy_root_systemd_unit_content(&self) {
        let buffer = &*self.imp().root_systemd_unit_content_buffer;
        buffer.select_range(&buffer.start_iter(), &buffer.end_iter());
        buffer.copy_clipboard(&gdk::Display::default().unwrap().clipboard());

        self.show_toast(&gettext("systemd unit content copied"));
    }

    fn copy_root_socket_acivation_command(&self) {
        let label = &*self.imp().root_socket_activation_command_label;
        label.select_region(0, -1);
        label.emit_copy_clipboard();

        self.show_toast(&gettext("socket activation command copied"));
    }

    fn copy_root_url(&self) {
        let label = &*self.imp().root_url_label;
        label.select_region(0, -1);
        label.emit_copy_clipboard();

        self.show_toast(&gettext("URL copied"));
    }

    fn show_toast(&self, title: &str) {
        self.imp().toast_overlay.add_toast(
            adw::Toast::builder()
                .timeout(2)
                .priority(adw::ToastPriority::High)
                .title(title)
                .build(),
        );
    }
}
