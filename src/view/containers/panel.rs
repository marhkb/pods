use adw::prelude::MessageDialogExtManual;
use adw::traits::BinExt;
use adw::traits::MessageDialogExt;
use gettextrs::gettext;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::subclass::Signal;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;
mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/containers/panel.ui")]
    pub(crate) struct Panel {
        pub(super) container_list: WeakRef<model::ContainerList>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) containers_group: TemplateChild<view::ContainersGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Panel {
        const NAME: &'static str = "PdsContainersPanel";
        type Type = super::Panel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                "containers.create",
                None,
            );
            klass.install_action("containers.create", None, move |widget, _, _| {
                widget.create_container();
            });

            klass.install_action(
                "containers-panel.start-or-resume-selection",
                None,
                move |widget, _, _| {
                    widget.start_or_resume_selection();
                },
            );
            klass.install_action(
                "containers-panel.stop-selection",
                None,
                move |widget, _, _| {
                    widget.stop_selection();
                },
            );
            klass.install_action(
                "containers-panel.pause-selection",
                None,
                move |widget, _, _| {
                    widget.pause_selection();
                },
            );
            klass.install_action(
                "containers-panel.restart-selection",
                None,
                move |widget, _, _| {
                    widget.restart_selection();
                },
            );
            klass.install_action(
                "containers-panel.delete-selection",
                None,
                move |widget, _, _| {
                    widget.delete_selection();
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Panel {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("exit-selection-mode", &[], <()>::static_type().into()).build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "container-list",
                    "Container List",
                    "The list of containers",
                    model::ContainerList::static_type(),
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
                "container-list" => obj.set_container_list(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container-list" => obj.container_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let container_list_expr = Self::Type::this_expression("container-list");
            let container_list_len_expr =
                container_list_expr.chain_property::<model::ContainerList>("len");

            gtk::ClosureExpression::new::<String, _, _>(
                &[
                    container_list_len_expr,
                    container_list_expr.chain_property::<model::ContainerList>("listing"),
                ],
                closure!(|_: Self::Type, len: u32, listing: bool| {
                    if len == 0 && listing {
                        "spinner"
                    } else {
                        "containers"
                    }
                }),
            )
            .bind(&*self.main_stack, "visible-child-name", Some(obj));
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.main_stack.unparent();
        }
    }

    impl WidgetImpl for Panel {}
}

glib::wrapper! {
    pub(crate) struct Panel(ObjectSubclass<imp::Panel>)
        @extends gtk::Widget;
}

impl Default for Panel {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create PdsContainersPanel")
    }
}

impl Panel {
    pub(crate) fn container_list(&self) -> Option<model::ContainerList> {
        self.imp().container_list.upgrade()
    }

    pub(crate) fn set_container_list(&self, value: &model::ContainerList) {
        if self.container_list().as_ref() == Some(value) {
            return;
        }

        self.action_set_enabled("containers-panel.start-or-resume-selection", false);
        self.action_set_enabled("containers-panel.stop-selection", false);
        self.action_set_enabled("containers-panel.pause-selection", false);
        self.action_set_enabled("containers-panel.restart-selection", false);
        self.action_set_enabled("containers-panel.delete-selection", false);
        value.connect_notify_local(
            Some("num-selected"),
            clone!(@weak self as obj => move |list, _| {
                let enabled = list.num_selected() > 0;
                obj.action_set_enabled("containers-panel.start-or-resume-selection", enabled);
                obj.action_set_enabled("containers-panel.stop-selection", enabled);
                obj.action_set_enabled("containers-panel.pause-selection", enabled);
                obj.action_set_enabled("containers-panel.restart-selection", enabled);
                obj.action_set_enabled("containers-panel.delete-selection", enabled);
            }),
        );

        self.imp().container_list.set(Some(value));
        self.notify("container-list");
    }

    fn create_container(&self) {
        let leaflet_overlay = utils::find_leaflet_overlay(self);

        if leaflet_overlay.child().is_none() {
            leaflet_overlay.show_details(&view::ContainerCreationPage::from(
                self.container_list()
                    .as_ref()
                    .and_then(model::ContainerList::client)
                    .as_ref(),
            ));
        }
    }

    fn start_or_resume_selection(&self) {
        if let Some(list) = self.container_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Container>().unwrap())
                .for_each(|container| {
                    match container.status() {
                        model::ContainerStatus::Paused => {
                            container.resume(clone!(@weak  self as obj => move |result| {
                                if let Err(e) = result {
                                    utils::show_toast(
                                        &obj,
                                        // Translators: The "{}" is a placeholder for an error message.
                                        &gettext!("Error on resuming container: {}", e)
                                    );
                                }
                            }));
                        }
                        other if other != model::ContainerStatus::Running => {
                            container.start(clone!(@weak  self as obj => move |result| {
                                if let Err(e) = result {
                                    utils::show_toast(
                                        &obj,
                                        // Translators: The "{}" is a placeholder for an error message.
                                        &gettext!("Error on starting container: {}", e)
                                    );
                                }
                            }));
                        }
                        _ => (),
                    }
                });
            list.set_selection_mode(false);
            self.emit_by_name::<()>("exit-selection-mode", &[]);
        }
    }

    fn stop_selection(&self) {
        if let Some(list) = self.container_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Container>().unwrap())
                .filter(|container| matches!(container.status(), model::ContainerStatus::Running))
                .for_each(|container| {
                    container.stop(
                        false,
                        clone!(@weak self as obj => move |result| {
                            if let Err(e) = result {
                                utils::show_toast(
                                    &obj,
                                    // Translators: The "{}" is a placeholder for an error message.
                                    &gettext!("Error on stopping container: {}", e)
                                );
                            }
                        }),
                    );
                });
            list.set_selection_mode(false);
            self.emit_by_name::<()>("exit-selection-mode", &[]);
        }
    }

    fn pause_selection(&self) {
        if let Some(list) = self.container_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Container>().unwrap())
                .filter(|container| matches!(container.status(), model::ContainerStatus::Running))
                .for_each(|container| {
                    container.pause(clone!(@weak self as obj => move |result| {
                        if let Err(e) = result {
                            utils::show_toast(
                                &obj,
                                // Translators: The "{}" is a placeholder for an error message.
                                &gettext!("Error on stopping container: {}", e)
                            );
                        }
                    }));
                });
            list.set_selection_mode(false);
            self.emit_by_name::<()>("exit-selection-mode", &[]);
        }
    }

    fn restart_selection(&self) {
        if let Some(list) = self.container_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Container>().unwrap())
                .filter(|container| matches!(container.status(), model::ContainerStatus::Running))
                .for_each(|container| {
                    container.restart(
                        false,
                        clone!(@weak self as obj => move |result| {
                            if let Err(e) = result {
                                utils::show_toast(
                                    &obj,
                                    // Translators: The "{}" is a placeholder for an error message.
                                    &gettext!("Error on restarting container: {}", e)
                                );
                            }
                        }),
                    );
                });
            list.set_selection_mode(false);
            self.emit_by_name::<()>("exit-selection-mode", &[]);
        }
    }

    fn delete_selection(&self) {
        if self
            .container_list()
            .map(|list| list.num_selected())
            .unwrap_or(0)
            == 0
        {
            return;
        }

        let dialog = adw::MessageDialog::builder()
            .heading(&gettext("Confirm Forced Deletion of Multiple Containers"))
            .body_use_markup(true)
            .body(&gettext(
                "All the data created inside the containers will be lost and running containers will be stopped!",
            ))
            .modal(true)
            .transient_for(&utils::root(self))
            .build();

        dialog.add_responses(&[
            ("cancel", &gettext("_Cancel")),
            ("delete", &gettext("_Delete")),
        ]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

        dialog.connect_response(
            None,
            clone!(@weak self as obj => move |_, response| if response == "delete" {
                if let Some(list) = obj.container_list() {
                    list
                        .selected_items()
                        .iter()
                        .map(|obj| obj.downcast_ref::<model::Container>().unwrap())
                        .for_each(|container|
                    {
                        container.delete(true, clone!(@weak obj => move |result| {
                            if let Err(e) = result {
                                utils::show_toast(
                                    &obj,
                                    // Translators: The "{}" is a placeholder for an error message.
                                    &gettext!("Error on deleting container: {}", e)
                                );
                            }
                        }));
                    });
                    list.set_selection_mode(false);
                    obj.emit_by_name::<()>("exit-selection-mode", &[]);
                }
            }),
        );

        dialog.present();
    }

    pub(crate) fn connect_exit_selection_mode<F: Fn(&Self) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("exit-selection-mode", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);

            None
        })
    }
}
