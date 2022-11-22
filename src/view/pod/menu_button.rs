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

const ACTION_CREATE_CONTAINER: &str = "pod-menu-button.create-container";
const ACTION_START: &str = "pod-menu-button.start";
const ACTION_STOP: &str = "pod-menu-button.stop";
const ACTION_KILL: &str = "pod-menu-button.kill";
const ACTION_RESTART: &str = "pod-menu-button.restart";
const ACTION_PAUSE: &str = "pod-menu-button.pause";
const ACTION_RESUME: &str = "pod-menu-button.resume";
const ACTION_DELETE: &str = "pod-menu-button.delete";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/pod/menu-button.ui")]
    pub(crate) struct MenuButton {
        pub(super) pod: glib::WeakRef<model::Pod>,
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
                super::super::start(widget.upcast_ref());
            });
            klass.install_action(ACTION_STOP, None, move |widget, _, _| {
                super::super::stop(widget.upcast_ref());
            });
            klass.install_action(ACTION_KILL, None, move |widget, _, _| {
                super::super::kill(widget.upcast_ref());
            });
            klass.install_action(ACTION_RESTART, None, move |widget, _, _| {
                super::super::restart(widget.upcast_ref());
            });
            klass.install_action(ACTION_PAUSE, None, move |widget, _, _| {
                super::super::pause(widget.upcast_ref());
            });
            klass.install_action(ACTION_RESUME, None, move |widget, _, _| {
                super::super::resume(widget.upcast_ref());
            });

            klass.install_action(ACTION_DELETE, None, move |widget, _, _| {
                super::super::show_delete_confirmation_dialog(widget.upcast_ref());
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MenuButton {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Pod>("pod")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecBoolean::builder("primary")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "pod" => obj.set_pod(value.get().unwrap_or_default()),
                "primary" => obj.set_primary(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "pod" => obj.pod().to_value(),
                "primary" => obj.is_primary().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            self.menu_button
                .connect_primary_notify(clone!(@weak obj => move |_| {
                    obj.notify("primary")
                }));

            Self::Type::this_expression("css-classes").bind(
                &*self.menu_button,
                "css-classes",
                Some(obj),
            );

            let pod_expr = Self::Type::this_expression("pod");

            pod_expr
                .chain_property::<model::Pod>("action-ongoing")
                .chain_closure::<bool>(closure!(|_: Self::Type, action_ongoing: bool| {
                    !action_ongoing
                }))
                .bind(&*self.menu_button, "sensitive", Some(obj));

            pod_expr
                .chain_property::<model::Pod>("status")
                .watch(Some(obj), clone!(@weak obj => move || obj.update_actions()));
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
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

    pub(crate) fn is_primary(&self) -> bool {
        self.imp().menu_button.is_primary()
    }

    pub(crate) fn set_primary(&self, value: bool) {
        self.imp().menu_button.set_primary(value);
    }

    pub(crate) fn create_container(&self) {
        if let Some(pod) = self.pod().as_ref() {
            utils::find_leaflet_overlay(self).show_details(&view::ContainerCreationPage::from(pod));
        }
    }

    fn update_actions(&self) {
        if let Some(pod) = self.pod() {
            let can_stop = pod.can_stop();

            self.action_set_enabled(ACTION_START, pod.can_start());
            self.action_set_enabled(ACTION_STOP, can_stop);
            self.action_set_enabled(ACTION_KILL, can_stop);
            self.action_set_enabled(ACTION_RESTART, pod.can_restart());
            self.action_set_enabled(ACTION_RESUME, pod.can_resume());
            self.action_set_enabled(ACTION_PAUSE, pod.can_pause());
            self.action_set_enabled(ACTION_DELETE, pod.can_delete());
        }
    }
}
