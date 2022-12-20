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

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container/menu-button.ui")]
    pub(crate) struct MenuButton {
        pub(super) container: glib::WeakRef<model::Container>,
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

            klass.install_action(ACTION_RENAME, None, move |widget, _, _| {
                widget.rename();
            });

            klass.install_action(ACTION_DELETE, None, move |widget, _, _| {
                super::super::delete(widget.upcast_ref());
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
                    glib::ParamSpecObject::builder::<model::Container>("container")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecBoolean::builder("primary")
                        .explicit_notify()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "container" => obj.set_container(value.get().unwrap_or_default()),
                "primary" => obj.set_primary(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "container" => obj.container().to_value(),
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

            let container_expr = Self::Type::this_expression("container");

            container_expr
                .chain_property::<model::Container>("action-ongoing")
                .chain_closure::<bool>(closure!(|_: Self::Type, action_ongoing: bool| {
                    !action_ongoing
                }))
                .bind(&*self.menu_button, "sensitive", Some(obj));

            container_expr
                .chain_property::<model::Container>("status")
                .watch(Some(obj), clone!(@weak obj => move || obj.update_actions()));
        }

        fn dispose(&self) {
            utils::ChildIter::from(self.obj().upcast_ref()).for_each(|child| child.unparent());
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

    pub(crate) fn is_primary(&self) -> bool {
        self.imp().menu_button.is_primary()
    }

    pub(crate) fn set_primary(&self, value: bool) {
        self.imp().menu_button.set_primary(value);
    }

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

    fn rename(&self) {
        if let Some(container) = self.container() {
            let dialog = view::ContainerRenameDialog::from(&container);
            dialog.set_transient_for(Some(&utils::root(self.upcast_ref())));
            dialog.present();
        }
    }
}
