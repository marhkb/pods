use gettextrs::gettext;
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

const ACTION_START: &str = "container-menu-button.start";
const ACTION_STOP: &str = "container-menu-button.stop";
const ACTION_FORCE_STOP: &str = "container-menu-button.force-stop";
const ACTION_RESTART: &str = "container-menu-button.restart";
const ACTION_FORCE_RESTART: &str = "container-menu-button.force-restart";
const ACTION_PAUSE: &str = "container-menu-button.pause";
const ACTION_RESUME: &str = "container-menu-button.resume";
const ACTION_RENAME: &str = "container-menu-button.rename";
const ACTION_COMMIT: &str = "container-menu-button.commit";
const ACTION_DELETE: &str = "container-menu-button.delete";
const ACTION_FORCE_DELETE: &str = "container-menu-button.force-delete";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container/menu-button.ui")]
    pub(crate) struct MenuButton {
        pub(super) container: WeakRef<model::Container>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MenuButton {
        const NAME: &'static str = "PdsContainerMenuButton";
        type Type = super::MenuButton;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_START, None, move |widget, _, _| {
                widget.start();
            });
            klass.install_action(ACTION_STOP, None, move |widget, _, _| {
                widget.stop();
            });
            klass.install_action(ACTION_FORCE_STOP, None, move |widget, _, _| {
                widget.force_stop();
            });
            klass.install_action(ACTION_RESTART, None, move |widget, _, _| {
                widget.restart();
            });
            klass.install_action(ACTION_FORCE_RESTART, None, move |widget, _, _| {
                widget.force_restart();
            });
            klass.install_action(ACTION_PAUSE, None, move |widget, _, _| {
                widget.pause();
            });
            klass.install_action(ACTION_RESUME, None, move |widget, _, _| {
                widget.resume();
            });

            klass.install_action(ACTION_RENAME, None, move |widget, _, _| {
                widget.rename();
            });

            klass.install_action(ACTION_COMMIT, None, move |widget, _, _| {
                widget.commit();
            });

            klass.install_action(ACTION_DELETE, None, move |widget, _, _| {
                widget.delete();
            });
            klass.install_action(ACTION_FORCE_DELETE, None, move |widget, _, _| {
                widget.force_delete();
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
                    "container",
                    "container",
                    "The Container of this menu button",
                    model::Container::static_type(),
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
                "container" => obj.set_container(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container" => obj.container().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            Self::Type::this_expression("css-classes").bind(
                &*self.menu_button,
                "css-classes",
                Some(obj),
            );

            let container_expr = Self::Type::this_expression("container");

            container_expr
                .chain_property::<model::Container>("action-ongoing")
                .chain_closure::<String>(closure!(|_: glib::Object, action_ongoing: bool| {
                    if action_ongoing {
                        "ongoing"
                    } else {
                        "menu"
                    }
                }))
                .bind(&*self.stack, "visible-child-name", Some(obj));

            container_expr
                .chain_property::<model::Container>("status")
                .watch(Some(obj), clone!(@weak obj => move || obj.update_actions()));
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

macro_rules! container_action {
    (fn $name:ident => $action:ident($($param:literal),*) => $error:tt) => {
        fn $name(&self) {
            if let Some(container) = self.container() {
                container.$action(
                    $($param,)*
                    clone!(@weak self as obj => move |result| if let Err(e) = result {
                        utils::show_error_toast(
                            &obj,
                            &gettext($error),
                            &e.to_string()
                        );
                    }),
                );
            }
        }
    };
}

impl MenuButton {
    pub(crate) fn popup(&self) {
        self.imp().menu_button.popup();
    }

    pub(crate) fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    pub(crate) fn set_container(&self, value: Option<&model::Container>) {
        if self.container().as_ref() == value {
            return;
        }
        self.imp().container.set(value);
        self.notify("container");
    }

    fn update_actions(&self) {
        use model::ContainerStatus::*;

        if let Some(container) = self.container() {
            let status = container.status();

            self.action_set_enabled(ACTION_START, matches!(status, Created | Exited));
            self.action_set_enabled(ACTION_STOP, matches!(status, Running));
            self.action_set_enabled(ACTION_FORCE_STOP, matches!(status, Running));
            self.action_set_enabled(ACTION_RESTART, matches!(status, Running));
            self.action_set_enabled(ACTION_FORCE_RESTART, matches!(status, Running));
            self.action_set_enabled(ACTION_RESUME, matches!(status, Paused));
            self.action_set_enabled(ACTION_PAUSE, matches!(status, Running));
            self.action_set_enabled(ACTION_DELETE, matches!(status, Created | Exited | Dead));
            self.action_set_enabled(ACTION_FORCE_DELETE, matches!(status, Running | Paused));
        }
    }

    container_action!(fn start => start() => "Error on starting container");
    container_action!(fn stop => stop(false) => "Error on stopping container");
    container_action!(fn force_stop => stop(true) => "Error on force stopping container");
    container_action!(fn restart => restart(false) => "Error on restarting container");
    container_action!(fn force_restart => restart(true) => "Error on force restarting container");
    container_action!(fn pause => pause() => "Error on pausing container");
    container_action!(fn resume => resume() => "Error on resuming container");
    container_action!(fn commit => commit() => "Error on committing container");
    container_action!(fn delete => delete(false) => "Error on deleting container");
    container_action!(fn force_delete => delete(true) => "Error on force deleting container");

    fn rename(&self) {
        let dialog = view::ContainerRenameDialog::from(self.container());
        dialog.set_transient_for(Some(&utils::root(self)));
        dialog.present();
    }
}
