use std::cell::RefCell;

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

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container-menu-button.ui")]
    pub(crate) struct ContainerMenuButton {
        pub(super) container: WeakRef<model::Container>,
        pub(super) status_handler_id: RefCell<Option<glib::SignalHandlerId>>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerMenuButton {
        const NAME: &'static str = "ContainerMenuButton";
        type Type = super::ContainerMenuButton;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("container.start", None, move |widget, _, _| {
                widget.start();
            });
            klass.install_action("container.stop", None, move |widget, _, _| {
                widget.stop();
            });
            klass.install_action("container.force-stop", None, move |widget, _, _| {
                widget.force_stop();
            });
            klass.install_action("container.restart", None, move |widget, _, _| {
                widget.restart();
            });
            klass.install_action("container.force-restart", None, move |widget, _, _| {
                widget.force_restart();
            });
            klass.install_action("container.pause", None, move |widget, _, _| {
                widget.pause();
            });
            klass.install_action("container.resume", None, move |widget, _, _| {
                widget.resume();
            });

            klass.install_action("container.rename", None, move |widget, _, _| {
                widget.rename();
            });

            klass.install_action("container.commit", None, move |widget, _, _| {
                widget.commit();
            });

            klass.install_action("container.delete", None, move |widget, _, _| {
                widget.delete();
            });
            klass.install_action("container.force-delete", None, move |widget, _, _| {
                widget.force_delete();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerMenuButton {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "container",
                    "container",
                    "The Container of this ContainerMenuButton",
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

            Self::Type::this_expression("container")
                .chain_property::<model::Container>("action-ongoing")
                .chain_closure::<String>(closure!(|_: glib::Object, action_ongoing: bool| {
                    if action_ongoing {
                        "ongoing"
                    } else {
                        "menu"
                    }
                }))
                .bind(&*self.stack, "visible-child-name", Some(obj));
        }

        fn dispose(&self, obj: &Self::Type) {
            let mut child = obj.first_child();
            while let Some(child_) = child {
                child = child_.next_sibling();
                child_.unparent();
            }
        }
    }

    impl WidgetImpl for ContainerMenuButton {}
}

glib::wrapper! {
    pub(crate) struct ContainerMenuButton(ObjectSubclass<imp::ContainerMenuButton>)
        @extends gtk::Widget;
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

impl ContainerMenuButton {
    pub(crate) fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    pub(crate) fn set_container(&self, value: Option<&model::Container>) {
        if self.container().as_ref() == value {
            return;
        }

        let imp = self.imp();

        if let Some(handler_id) = imp.status_handler_id.take() {
            if let Some(container) = self.container() {
                container.disconnect(handler_id);
            }
        }

        if let Some(container) = value {
            self.update_actions(container);

            let status_handler_id = container.connect_notify_local(
                Some("status"),
                clone!(@weak self as obj => move |container, _| obj.update_actions(container)),
            );
            imp.status_handler_id.replace(Some(status_handler_id));
        }

        imp.container.set(value);
        self.notify("container");
    }

    fn update_actions(&self, container: &model::Container) {
        use model::ContainerStatus::*;

        match container.status() {
            Running => {
                self.action_set_enabled("container.start", false);
                self.action_set_enabled("container.stop", true);
                self.action_set_enabled("container.force-stop", true);
                self.action_set_enabled("container.restart", true);
                self.action_set_enabled("container.force-restart", true);
                self.action_set_enabled("container.resume", false);
                self.action_set_enabled("container.pause", true);
                self.action_set_enabled("container.delete", false);
                self.action_set_enabled("container.force-delete", true);
            }
            Paused => {
                self.action_set_enabled("container.start", false);
                self.action_set_enabled("container.stop", false);
                self.action_set_enabled("container.force-stop", false);
                self.action_set_enabled("container.restart", false);
                self.action_set_enabled("container.force-restart", false);
                self.action_set_enabled("container.resume", true);
                self.action_set_enabled("container.pause", false);
                self.action_set_enabled("container.delete", false);
                self.action_set_enabled("container.force-delete", true);
            }
            Configured | Created | Exited | Dead | Stopped => {
                self.action_set_enabled("container.start", true);
                self.action_set_enabled("container.stop", false);
                self.action_set_enabled("container.force-stop", false);
                self.action_set_enabled("container.restart", false);
                self.action_set_enabled("container.force-restart", false);
                self.action_set_enabled("container.resume", false);
                self.action_set_enabled("container.pause", false);
                self.action_set_enabled("container.delete", true);
                self.action_set_enabled("container.force-delete", false);
            }
            _ => {
                self.action_set_enabled("container.start", false);
                self.action_set_enabled("container.stop", false);
                self.action_set_enabled("container.force-stop", false);
                self.action_set_enabled("container.restart", false);
                self.action_set_enabled("container.force-restart", false);
                self.action_set_enabled("container.resume", false);
                self.action_set_enabled("container.pause", false);
                self.action_set_enabled("container.delete", false);
                self.action_set_enabled("container.force-delete", false);
            }
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
        dialog.set_transient_for(Some(
            &self.root().unwrap().downcast::<gtk::Window>().unwrap(),
        ));
        dialog.run_async(clone!(@weak self as obj => move |dialog, response| {
            obj.on_rename_dialog_response(dialog.upcast_ref(), response, |_, dialog| {
                dialog.connect_response(clone!(@weak obj => move |dialog, response| {
                    obj.on_rename_dialog_response(dialog, response, |_, _| {});
                }));
            });
        }));
    }

    fn on_rename_dialog_response<F>(&self, dialog: &gtk::Dialog, response: gtk::ResponseType, op: F)
    where
        F: Fn(&Self, &gtk::Dialog),
    {
        match response {
            gtk::ResponseType::Cancel | gtk::ResponseType::Apply => dialog.close(),
            _ => op(self, dialog),
        }
    }
}
