use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_START: &str = "container-menu-button.start";
const ACTION_STOP: &str = "container-menu-button.stop";
const ACTION_KILL: &str = "container-menu-button.kill";
const ACTION_RESTART: &str = "container-menu-button.restart";
const ACTION_PAUSE: &str = "container-menu-button.pause";
const ACTION_RESUME: &str = "container-menu-button.resume";
const ACTION_RENAME: &str = "container-menu-button.rename";
const ACTION_DELETE: &str = "container-menu-button.delete";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerMenuButton)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_menu_button.ui")]
    pub(crate) struct ContainerMenuButton {
        #[property(get, set, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerMenuButton {
        const NAME: &'static str = "PdsContainerMenuButton";
        type Type = super::ContainerMenuButton;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_START, None, |widget, _, _| {
                view::container::start(widget, widget.container());
            });
            klass.install_action(ACTION_STOP, None, |widget, _, _| {
                view::container::stop(widget, widget.container());
            });
            klass.install_action(ACTION_KILL, None, |widget, _, _| {
                view::container::kill(widget, widget.container());
            });
            klass.install_action(ACTION_RESTART, None, |widget, _, _| {
                view::container::restart(widget, widget.container());
            });
            klass.install_action(ACTION_PAUSE, None, |widget, _, _| {
                view::container::pause(widget, widget.container());
            });
            klass.install_action(ACTION_RESUME, None, |widget, _, _| {
                view::container::resume(widget, widget.container());
            });

            klass.install_action(ACTION_RENAME, None, |widget, _, _| {
                view::container::rename(widget, widget.container().as_ref());
            });

            klass.install_action(ACTION_DELETE, None, |widget, _, _| {
                view::container::delete(widget, widget.container());
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerMenuButton {
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

            let container_expr = Self::Type::this_expression("container");

            container_expr
                .chain_property::<model::Container>("action-ongoing")
                .chain_closure::<bool>(closure!(|_: Self::Type, action_ongoing: bool| {
                    !action_ongoing
                }))
                .bind(&*self.menu_button, "sensitive", Some(obj));

            container_expr
                .chain_property::<model::Container>("status")
                .watch(
                    Some(obj),
                    clone!(
                        #[weak]
                        obj,
                        move || obj.update_actions()
                    ),
                );
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ContainerMenuButton {}
}

glib::wrapper! {
    pub(crate) struct ContainerMenuButton(ObjectSubclass<imp::ContainerMenuButton>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ContainerMenuButton {
    fn update_actions(&self) {
        if let Some(container) = self.container() {
            let can_stop = container.can_stop();

            self.action_set_enabled(ACTION_START, container.can_start());
            self.action_set_enabled(ACTION_STOP, can_stop);
            self.action_set_enabled(ACTION_KILL, can_stop);
            self.action_set_enabled(ACTION_RESTART, container.can_restart());
            self.action_set_enabled(ACTION_RESUME, container.can_resume());
            self.action_set_enabled(ACTION_PAUSE, container.can_pause());
            self.action_set_enabled(ACTION_DELETE, container.can_delete());
        }
    }
}
