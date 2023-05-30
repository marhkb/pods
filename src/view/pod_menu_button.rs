use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

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

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PodMenuButton)]
    #[template(file = "pod_menu_button.ui")]
    pub(crate) struct PodMenuButton {
        #[property(get, set, nullable)]
        pub(super) pod: glib::WeakRef<model::Pod>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodMenuButton {
        const NAME: &'static str = "PdsPodMenuButton";
        type Type = super::PodMenuButton;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_CREATE_CONTAINER, None, |widget, _, _| {
                widget.create_container();
            });

            klass.install_action(ACTION_START, None, |widget, _, _| {
                view::pod::start(widget.upcast_ref());
            });
            klass.install_action(ACTION_STOP, None, |widget, _, _| {
                view::pod::stop(widget.upcast_ref());
            });
            klass.install_action(ACTION_KILL, None, |widget, _, _| {
                view::pod::kill(widget.upcast_ref());
            });
            klass.install_action(ACTION_RESTART, None, |widget, _, _| {
                view::pod::restart(widget.upcast_ref());
            });
            klass.install_action(ACTION_PAUSE, None, |widget, _, _| {
                view::pod::pause(widget.upcast_ref());
            });
            klass.install_action(ACTION_RESUME, None, |widget, _, _| {
                view::pod::resume(widget.upcast_ref());
            });

            klass.install_action(ACTION_DELETE, None, |widget, _, _| {
                view::pod::show_delete_confirmation_dialog(widget.upcast_ref());
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodMenuButton {
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
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for PodMenuButton {}
}

glib::wrapper! {
    pub(crate) struct PodMenuButton(ObjectSubclass<imp::PodMenuButton>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PodMenuButton {
    pub(crate) fn create_container(&self) {
        view::pod::create_container(self.upcast_ref(), self.pod());
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
