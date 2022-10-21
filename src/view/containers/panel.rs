use adw::prelude::MessageDialogExtManual;
use adw::traits::BinExt;
use adw::traits::MessageDialogExt;
use gettextrs::gettext;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::subclass::Signal;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

const ACTION_START_OR_RESUME_SELECTION: &str = "containers-panel.start-or-resume-selection";
const ACTION_STOP_SELECTION: &str = "containers-panel.stop-selection";
const ACTION_PAUSE_SELECTION: &str = "containers-panel.pause-selection";
const ACTION_RESTART_SELECTION: &str = "containers-panel.restart-selection";
const ACTION_DELETE_SELECTION: &str = "containers-panel.delete-selection";

const ACTIONS_SELECTION: &[&str] = &[
    ACTION_START_OR_RESUME_SELECTION,
    ACTION_STOP_SELECTION,
    ACTION_PAUSE_SELECTION,
    ACTION_RESTART_SELECTION,
    ACTION_DELETE_SELECTION,
];

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/containers/panel.ui")]
    pub(crate) struct Panel {
        pub(super) container_list: glib::WeakRef<model::ContainerList>,
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
                view::ContainersGroup::action_create_container(),
                None,
            );
            klass.install_action(
                view::ContainersGroup::action_create_container(),
                None,
                move |widget, _, _| {
                    widget.create_container();
                },
            );

            klass.install_action(
                ACTION_START_OR_RESUME_SELECTION,
                None,
                move |widget, _, _| {
                    widget.start_or_resume_selection();
                },
            );
            klass.install_action(ACTION_STOP_SELECTION, None, move |widget, _, _| {
                widget.stop_selection();
            });
            klass.install_action(ACTION_PAUSE_SELECTION, None, move |widget, _, _| {
                widget.pause_selection();
            });
            klass.install_action(ACTION_RESTART_SELECTION, None, move |widget, _, _| {
                widget.restart_selection();
            });
            klass.install_action(ACTION_DELETE_SELECTION, None, move |widget, _, _| {
                widget.delete_selection();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Panel {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("exit-selection-mode").build()]);
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::ContainerList>("container-list")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "container-list" => self.instance().set_container_list(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container-list" => self.instance().container_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            let container_list_expr = Self::Type::this_expression("container-list");
            let container_list_len_expr =
                container_list_expr.chain_property::<model::ContainerList>("len");

            container_list_len_expr.watch(
                Some(obj),
                clone!(@weak obj => move || {
                    let list = obj.container_list().unwrap();
                    if list.is_selection_mode() && list.len() == 0 {
                        list.set_selection_mode(false);
                        obj.emit_by_name::<()>("exit-selection-mode", &[]);
                    }
                }),
            );

            gtk::ClosureExpression::new::<Option<String>>(
                &[
                    container_list_len_expr,
                    container_list_expr.chain_property::<model::ContainerList>("listing"),
                    container_list_expr.chain_property::<model::ContainerList>("initialized"),
                ],
                closure!(
                    |_: Self::Type, len: u32, listing: bool, initialized: bool| {
                        if len == 0 {
                            if initialized {
                                Some("empty")
                            } else if listing {
                                Some("spinner")
                            } else {
                                None
                            }
                        } else {
                            Some("containers")
                        }
                    }
                ),
            )
            .bind(&*self.main_stack, "visible-child-name", Some(obj));
        }

        fn dispose(&self) {
            self.main_stack.unparent();
        }
    }

    impl WidgetImpl for Panel {}
}

glib::wrapper! {
    pub(crate) struct Panel(ObjectSubclass<imp::Panel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Panel {
    fn default() -> Self {
        glib::Object::builder::<Self>().build()
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

        ACTIONS_SELECTION
            .iter()
            .for_each(|action_name| self.action_set_enabled(action_name, false));
        value.connect_notify_local(
            Some("num-selected"),
            clone!(@weak self as obj => move |list, _| {
                ACTIONS_SELECTION
                    .iter()
                    .for_each(|action_name| {
                        obj.action_set_enabled(action_name, list.num_selected() > 0);
                });
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
