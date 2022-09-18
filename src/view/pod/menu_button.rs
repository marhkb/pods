use adw::prelude::MessageDialogExtManual;
use adw::traits::MessageDialogExt;
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

const ACTION_CREATE_CONTAINER: &str = "pod-menu-button.create-container";
const ACTION_START: &str = "pod-menu-button.start";
const ACTION_STOP: &str = "pod-menu-button.stop";
const ACTION_FORCE_STOP: &str = "pod-menu-button.force-stop";
const ACTION_RESTART: &str = "pod-menu-button.restart";
const ACTION_FORCE_RESTART: &str = "pod-menu-button.force-restart";
const ACTION_PAUSE: &str = "pod-menu-button.pause";
const ACTION_RESUME: &str = "pod-menu-button.resume";
const ACTION_DELETE: &str = "pod-menu-button.delete";
const ACTION_FORCE_DELETE: &str = "pod-menu-button.force-delete";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/pod/menu-button.ui")]
    pub(crate) struct MenuButton {
        pub(super) pod: WeakRef<model::Pod>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MenuButton {
        const NAME: &'static str = "PdsPodMenuButton";
        type Type = super::MenuButton;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_CREATE_CONTAINER, None, move |widget, _, _| {
                widget.create_container();
            });

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

            klass.install_action(ACTION_DELETE, None, move |widget, _, _| {
                widget.show_delete_confirmation_dialog(false);
            });
            klass.install_action(ACTION_FORCE_DELETE, None, move |widget, _, _| {
                widget.show_delete_confirmation_dialog(true);
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
                    "pod",
                    "Pod",
                    "The pod of this pod menu button",
                    model::Pod::static_type(),
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
                "pod" => obj.set_pod(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "pod" => obj.pod().to_value(),
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

            let pod_expr = Self::Type::this_expression("pod");

            pod_expr
                .chain_property::<model::Pod>("action-ongoing")
                .chain_closure::<String>(closure!(|_: Self::Type, action_ongoing: bool| {
                    if action_ongoing {
                        "ongoing"
                    } else {
                        "menu"
                    }
                }))
                .bind(&*self.stack, "visible-child-name", Some(obj));

            pod_expr
                .chain_property::<model::Pod>("status")
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

macro_rules! pod_action {
    (fn $name:ident => $action:ident($($param:literal),*) => $error:tt) => {
        fn $name(&self) {
            if let Some(pod) = self.pod() {
                pod.$action(
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

    pub(crate) fn pod(&self) -> Option<model::Pod> {
        self.imp().pod.upgrade()
    }

    pub(crate) fn set_pod(&self, value: Option<&model::Pod>) {
        if self.pod().as_ref() == value {
            return;
        }
        self.imp().pod.set(value);
        self.notify("pod");
    }

    pub(crate) fn create_container(&self) {
        if let Some(pod) = self.pod().as_ref() {
            utils::find_leaflet_overlay(self).show_details(&view::ContainerCreationPage::from(pod));
        }
    }

    fn update_actions(&self) {
        use model::PodStatus::*;

        if let Some(pod) = self.pod() {
            let status = pod.status();

            self.action_set_enabled(
                ACTION_START,
                matches!(pod.status(), Created | Exited | Dead),
            );
            self.action_set_enabled(ACTION_STOP, matches!(status, Running));
            self.action_set_enabled(ACTION_FORCE_STOP, matches!(status, Running));
            self.action_set_enabled(ACTION_RESTART, matches!(status, Running));
            self.action_set_enabled(ACTION_FORCE_RESTART, matches!(status, Running));
            self.action_set_enabled(ACTION_RESUME, matches!(status, Paused));
            self.action_set_enabled(ACTION_PAUSE, matches!(status, Running));
            self.action_set_enabled(
                ACTION_DELETE,
                matches!(status, Created | Exited | Dead | Degraded),
            );
            self.action_set_enabled(ACTION_FORCE_DELETE, matches!(status, Running | Paused));
        }
    }

    pod_action!(fn start => start() => "Error on starting pod");
    pod_action!(fn stop => stop(false) => "Error on stopping pod");
    pod_action!(fn force_stop => stop(true) => "Error on force stopping pod");
    pod_action!(fn restart => restart(false) => "Error on restarting pod");
    pod_action!(fn force_restart => restart(true) => "Error on force restarting pod");
    pod_action!(fn pause => pause() => "Error on pausing pod");
    pod_action!(fn resume => resume() => "Error on resuming pod");
    pod_action!(fn delete => delete(false) => "Error on deleting pod");
    pod_action!(fn force_delete => delete(true) => "Error on force deleting pod");

    fn delete_(&self, force: bool) {
        if force {
            self.force_delete();
        } else {
            self.delete();
        }
    }

    fn show_delete_confirmation_dialog(&self, force: bool) {
        if let Some(pod) = self.pod().as_ref() {
            let first_container = pod.container_list().get(0);

            if pod.num_containers() > 0 || first_container.is_some() {
                let dialog = adw::MessageDialog::builder()
                    .heading(&gettext("Confirm Forced Pod Deletion"))
                    .body_use_markup(true)
                    .body(
                        &match first_container.as_ref().map(|c| c.name()) {
                            Some(id) => gettext!(
                                // Translators: The "{}" is a placeholder for the container name.
                                "Pod contains container <b>{}</b>. Deleting the pod will also delete all its containers.",
                                id
                            ),
                            None => gettext(
                               "Pod conains a container. Deleting the pod will also delete all its containers.",
                           ),
                        }

                    )
                    .modal(true)
                    .transient_for(&utils::root(self))
                    .build();

                dialog.add_responses(&[
                    ("cancel", &gettext("_Cancel")),
                    ("delete", &gettext("_Force Delete")),
                ]);
                dialog.set_default_response(Some("cancel"));
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

                dialog.connect_response(
                    None,
                    clone!(@weak self as obj, @weak pod => move |_, response| {
                        if response == "delete" {
                            obj.delete_(force);
                        }
                    }),
                );

                dialog.present();
            } else {
                self.delete_(force);
            }
        }
    }
}
