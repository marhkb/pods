use gtk::glib::{clone, closure, WeakRef};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib, CompositeTemplate};
use once_cell::sync::Lazy;

use crate::window::Window;
use crate::{model, utils, view};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container-page.ui")]
    pub(crate) struct ContainerPage {
        pub(super) container: WeakRef<model::Container>,
        #[template_child]
        pub(super) leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) logs_show_timestamps_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) logs_search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) panel_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub(super) logs_panel: TemplateChild<view::ContainerLogsPanel>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerPage {
        const NAME: &'static str = "ContainerPage";
        type Type = super::ContainerPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("navigation.go-first", None, move |widget, _, _| {
                widget.navigate_to_first();
            });
            klass.install_action("navigation.back", None, move |widget, _, _| {
                widget.navigate_back();
            });

            klass.add_binding_action(
                gdk::Key::F,
                gdk::ModifierType::CONTROL_MASK,
                "logs.toggle-search",
                None,
            );
            klass.install_action("logs.toggle-search", None, |widget, _, _| {
                widget.toggle_logs_search();
            });

            klass.add_binding_action(
                gdk::Key::G,
                gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
                "logs.search-backward",
                None,
            );
            klass.install_action("logs.search-backward", None, |widget, _, _| {
                widget.search_logs_backward();
            });

            klass.add_binding_action(
                gdk::Key::G,
                gdk::ModifierType::CONTROL_MASK,
                "logs.search-forward",
                None,
            );
            klass.install_action("logs.search-forward", None, |widget, _, _| {
                widget.search_logs_forward();
            });

            klass.install_action("container.start", None, move |widget, _, _| {
                super::super::start(widget.upcast_ref(), &widget.container().unwrap());
            });
            klass.install_action("container.stop", None, move |widget, _, _| {
                super::super::stop(widget.upcast_ref(), &widget.container().unwrap());
            });
            klass.install_action("container.force-stop", None, move |widget, _, _| {
                super::super::force_stop(widget.upcast_ref(), &widget.container().unwrap());
            });
            klass.install_action("container.restart", None, move |widget, _, _| {
                super::super::restart(widget.upcast_ref(), &widget.container().unwrap());
            });
            klass.install_action("container.force-restart", None, move |widget, _, _| {
                super::super::force_restart(widget.upcast_ref(), &widget.container().unwrap());
            });
            klass.install_action("container.pause", None, move |widget, _, _| {
                super::super::pause(widget.upcast_ref(), &widget.container().unwrap());
            });
            klass.install_action("container.resume", None, move |widget, _, _| {
                super::super::resume(widget.upcast_ref(), &widget.container().unwrap());
            });

            klass.install_action("container.rename", None, move |widget, _, _| {
                super::super::rename(widget.upcast_ref(), widget.container());
            });

            klass.install_action("container.commit", None, move |widget, _, _| {
                super::super::commit(widget.upcast_ref(), &widget.container().unwrap());
            });

            klass.install_action("container.delete", None, move |widget, _, _| {
                super::super::delete(widget.upcast_ref(), &widget.container().unwrap());
            });
            klass.install_action("container.force-delete", None, move |widget, _, _| {
                super::super::force_delete(widget.upcast_ref(), &widget.container().unwrap());
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "container",
                    "Container",
                    "The container of this ContainerPage",
                    model::Container::static_type(),
                    glib::ParamFlags::READWRITE
                        | glib::ParamFlags::CONSTRUCT
                        | glib::ParamFlags::EXPLICIT_NOTIFY,
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
                "container" => obj.set_container(value.get().unwrap()),
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

            self.logs_panel
                .connect_show_timestamp_button(&*self.logs_show_timestamps_button);

            self.logs_panel
                .connect_search_button(&*self.logs_search_button);

            obj.update_buttons_and_actions(false);
            self.panel_stack.connect_visible_child_name_notify(
                clone!(@weak obj => move |panel_stack| {
                    obj.update_buttons_and_actions(
                        matches!(panel_stack.visible_child_name().as_deref(), Some("logs")),
                    );
                }),
            );

            let container_expr = Self::Type::this_expression("container");

            container_expr
                .chain_property::<model::Container>("deleted")
                .chain_closure::<String>(closure!(|_: Self::Type, deleted: bool| {
                    if deleted {
                        "deleted"
                    } else {
                        "container"
                    }
                }))
                .bind(&*self.stack, "visible-child-name", Some(obj));

            container_expr
                .chain_property::<model::Container>("status")
                .chain_closure::<Option<gio::MenuModel>>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| {
                        use model::ContainerStatus::*;

                        Some(match status {
                            Running => super::super::running_menu(),
                            Paused => super::super::paused_menu(),
                            Configured | Created | Exited | Dead | Stopped => {
                                super::super::stopped_menu()
                            }
                            _ => return None,
                        })
                    }
                ))
                .bind(&*self.menu_button, "menu-model", Some(obj));
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.leaflet.unparent();
        }
    }

    impl WidgetImpl for ContainerPage {
        fn realize(&self, widget: &Self::Type) {
            self.parent_realize(widget);

            widget.action_set_enabled(
                "navigation.go-first",
                widget.previous_leaflet_overlay() != widget.root_leaflet_overlay(),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerPage(ObjectSubclass<imp::ContainerPage>) @extends gtk::Widget;
}

impl From<&model::Container> for ContainerPage {
    fn from(image: &model::Container) -> Self {
        glib::Object::new(&[("container", image)]).expect("Failed to create ContainerPage")
    }
}

impl ContainerPage {
    fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    fn set_container(&self, value: Option<&model::Container>) {
        if self.container().as_ref() == value {
            return;
        }

        self.imp().container.set(value);
        self.notify("container");
    }

    fn navigate_to_first(&self) {
        self.root_leaflet_overlay().hide_details();
    }

    fn navigate_back(&self) {
        self.previous_leaflet_overlay().hide_details();
    }

    fn previous_leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::find_leaflet_overlay(self)
    }

    fn root_leaflet_overlay(&self) -> view::LeafletOverlay {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .leaflet_overlay()
    }

    fn update_buttons_and_actions(&self, logs_visible: bool) {
        self.imp()
            .logs_show_timestamps_button
            .set_visible(logs_visible);
        self.imp().logs_search_button.set_visible(logs_visible);

        self.action_set_enabled("logs.toggle-search", logs_visible);
        self.action_set_enabled("logs.search-backward", logs_visible);
        self.action_set_enabled("logs.search-forward", logs_visible);
    }

    fn toggle_logs_search(&self) {
        self.imp().logs_panel.toggle_search();
    }

    fn search_logs_backward(&self) {
        self.imp().logs_panel.search_backward();
    }

    fn search_logs_forward(&self) {
        self.imp().logs_panel.search_forward();
    }
}
