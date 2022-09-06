use adw::prelude::MessageDialogExtManual;
use adw::traits::BinExt;
use adw::traits::MessageDialogExt;
use gettextrs::gettext;
use gettextrs::ngettext;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::subclass::Signal;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;

use crate::model;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/pods-panel.ui")]
    pub(crate) struct PodsPanel {
        pub(super) settings: utils::PodsSettings,
        pub(super) pod_list: WeakRef<model::PodList>,
        pub(super) properties_filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) pods_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) show_only_running_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) create_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodsPanel {
        const NAME: &'static str = "PodsPanel";
        type Type = super::PodsPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                "pods.create",
                None,
            );
            klass.install_action("pods.create", None, move |widget, _, _| {
                widget.create_pod();
            });

            klass.install_action(
                "pods-panel.start-or-resume-selection",
                None,
                move |widget, _, _| {
                    widget.start_selection();
                },
            );
            klass.install_action("pods-panel.stop-selection", None, move |widget, _, _| {
                widget.stop_selection();
            });
            klass.install_action("pods-panel.pause-selection", None, move |widget, _, _| {
                widget.pause_selection();
            });
            klass.install_action("pods-panel.restart-selection", None, move |widget, _, _| {
                widget.restart_selection();
            });
            klass.install_action("pods-panel.delete-selection", None, move |widget, _, _| {
                widget.delete_selection();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodsPanel {
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
                    "pod-list",
                    "Pod List",
                    "The list of pods",
                    model::PodList::static_type(),
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
                "pod-list" => obj.set_pod_list(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "pod-list" => obj.pod_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            self.settings.connect_changed(
                Some("show-only-running-pods"),
                clone!(@weak obj => move |_, _| obj.update_properties_filter()),
            );
            self.settings
                .bind(
                    "show-only-running-pods",
                    &*self.show_only_running_switch,
                    "active",
                )
                .build();

            let pod_list_expr = Self::Type::this_expression("pod-list");
            let pod_list_len_expr = pod_list_expr.chain_property::<model::PodList>("len");

            pod_list_expr
                .chain_property::<model::PodList>("selection-mode")
                .chain_closure::<bool>(closure!(|_: Self::Type, selection_mode: bool| {
                    !selection_mode
                }))
                .bind(&*self.create_button, "visible", Some(obj));

            gtk::ClosureExpression::new::<String, _, _>(
                &[
                    pod_list_len_expr.as_ref(),
                    pod_list_expr
                        .chain_property::<model::PodList>("listing")
                        .as_ref(),
                ],
                closure!(|_: Self::Type, len: u32, listing: bool| {
                    if len == 0 && listing {
                        "spinner"
                    } else {
                        "pods"
                    }
                }),
            )
            .bind(&*self.main_stack, "visible-child-name", Some(obj));

            gtk::ClosureExpression::new::<Option<String>, _, _>(
                &[
                    pod_list_len_expr,
                    pod_list_expr.chain_property::<model::PodList>("running"),
                ],
                closure!(|_: Self::Type, len: u32, running: u32| {
                    if len == 0 {
                        gettext("No pods found")
                    } else if len == 1 {
                        if running == 1 {
                            gettext("1 pod, running")
                        } else {
                            gettext("1 pod, stopped")
                        }
                    } else {
                        ngettext!(
                            // Translators: There's a wide space (U+2002) between ", {}".
                            "{} pod total, {} running",
                            "{} pods total, {} running",
                            len,
                            len,
                            running,
                        )
                    }
                }),
            )
            .bind(&*self.pods_group, "description", Some(obj));

            let properties_filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    !obj.imp().show_only_running_switch.is_active() ||
                        item.downcast_ref::<model::Pod>().unwrap().status()
                            == model::PodStatus::Running
                }));

            let sorter = gtk::CustomSorter::new(|obj1, obj2| {
                let pod1 = obj1.downcast_ref::<model::Pod>().unwrap();
                let pod2 = obj2.downcast_ref::<model::Pod>().unwrap();

                pod1.name().cmp(&pod2.name()).into()
            });

            self.properties_filter
                .set(properties_filter.upcast())
                .unwrap();
            self.sorter.set(sorter.upcast()).unwrap();
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.main_stack.unparent();
        }
    }

    impl WidgetImpl for PodsPanel {}
}

glib::wrapper! {
    pub(crate) struct PodsPanel(ObjectSubclass<imp::PodsPanel>)
        @extends gtk::Widget;
}

impl Default for PodsPanel {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create PodsPanel")
    }
}

impl PodsPanel {
    pub(crate) fn pod_list(&self) -> Option<model::PodList> {
        self.imp().pod_list.upgrade()
    }

    pub(crate) fn set_pod_list(&self, value: &model::PodList) {
        if self.pod_list().as_ref() == Some(value) {
            return;
        }

        // TODO: For multi-client: Figure out whether signal handlers need to be disconnected.
        let imp = self.imp();

        value.connect_notify_local(
            Some("running"),
            clone!(@weak self as obj => move |_ ,_| obj.update_properties_filter()),
        );

        let model = gtk::SortListModel::new(
            Some(&gtk::FilterListModel::new(
                Some(value),
                imp.properties_filter.get(),
            )),
            imp.sorter.get(),
        );

        self.set_list_box_visibility(model.upcast_ref());
        model.connect_items_changed(clone!(@weak self as obj => move |model, _, _, _| {
            obj.set_list_box_visibility(model.upcast_ref());
        }));

        imp.list_box.bind_model(Some(&model), |item| {
            view::PodRow::from(item.downcast_ref().unwrap()).upcast()
        });

        self.action_set_enabled("pods-panel.start-or-resume-selection", false);
        self.action_set_enabled("pods-panel.stop-selection", false);
        self.action_set_enabled("pods-panel.pause-selection", false);
        self.action_set_enabled("pods-panel.restart-selection", false);
        self.action_set_enabled("pods-panel.delete-selection", false);
        value.connect_notify_local(
            Some("num-selected"),
            clone!(@weak self as obj => move |list, _| {
                let enabled = list.num_selected() > 0;
                obj.action_set_enabled("pods-panel.start-or-resume-selection", enabled);
                obj.action_set_enabled("pods-panel.stop-selection", enabled);
                obj.action_set_enabled("pods-panel.pause-selection", enabled);
                obj.action_set_enabled("pods-panel.restart-selection", enabled);
                obj.action_set_enabled("pods-panel.delete-selection", enabled);
            }),
        );

        imp.pod_list.set(Some(value));
        self.notify("pod-list");
    }

    fn set_list_box_visibility(&self, model: &gio::ListModel) {
        self.imp().list_box.set_visible(model.n_items() > 0);
    }

    pub(crate) fn update_properties_filter(&self) {
        self.imp()
            .properties_filter
            .get()
            .unwrap()
            .changed(gtk::FilterChange::Different);
    }

    fn create_pod(&self) {
        let leaflet_overlay = utils::find_leaflet_overlay(self);

        if leaflet_overlay.child().is_none() {
            leaflet_overlay.show_details(&view::PodCreationPage::from(
                self.pod_list()
                    .as_ref()
                    .and_then(model::PodList::client)
                    .as_ref(),
            ));
        }
    }

    fn start_selection(&self) {
        if let Some(list) = self.pod_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Pod>().unwrap())
                .for_each(|pod| {
                    match pod.status() {
                        model::PodStatus::Paused => {
                            pod.resume(clone!(@weak  self as obj => move |result| {
                                if let Err(e) = result {
                                    utils::show_toast(
                                        &obj,
                                        // Translators: The "{}" is a placeholder for an error message.
                                        &gettext!("Error on resuming pod: {}", e)
                                    );
                                }
                            }));
                        }
                        other if other != model::PodStatus::Running => {
                            pod.start(clone!(@weak  self as obj => move |result| {
                                if let Err(e) = result {
                                    utils::show_toast(
                                        &obj,
                                        // Translators: The "{}" is a placeholder for an error message.
                                        &gettext!("Error on starting pod: {}", e)
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
        if let Some(list) = self.pod_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Pod>().unwrap())
                .filter(|pod| matches!(pod.status(), model::PodStatus::Running))
                .for_each(|pod| {
                    pod.stop(
                        false,
                        clone!(@weak self as obj => move |result| {
                            if let Err(e) = result {
                                utils::show_toast(
                                    &obj,
                                    // Translators: The "{}" is a placeholder for an error message.
                                    &gettext!("Error on stopping pod: {}", e)
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
        if let Some(list) = self.pod_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Pod>().unwrap())
                .filter(|pod| matches!(pod.status(), model::PodStatus::Running))
                .for_each(|pod| {
                    pod.pause(clone!(@weak self as obj => move |result| {
                        if let Err(e) = result {
                            utils::show_toast(
                                &obj,
                                // Translators: The "{}" is a placeholder for an error message.
                                &gettext!("Error on stopping pod: {}", e)
                            );
                        }
                    }));
                });
            list.set_selection_mode(false);
            self.emit_by_name::<()>("exit-selection-mode", &[]);
        }
    }

    fn restart_selection(&self) {
        if let Some(list) = self.pod_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Pod>().unwrap())
                .filter(|pod| matches!(pod.status(), model::PodStatus::Running))
                .for_each(|pod| {
                    pod.restart(
                        false,
                        clone!(@weak self as obj => move |result| {
                            if let Err(e) = result {
                                utils::show_toast(
                                    &obj,
                                    // Translators: The "{}" is a placeholder for an error message.
                                    &gettext!("Error on restarting pod: {}", e)
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
        if self.pod_list().map(|list| list.num_selected()).unwrap_or(0) == 0 {
            return;
        }

        let dialog = adw::MessageDialog::builder()
            .heading(&gettext("Confirm Forced Deletion of Multiple Pods"))
            .body_use_markup(true)
            .body(&gettext("All associated containers will also be removed!"))
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
                if let Some(list) = obj.pod_list() {
                    list
                        .selected_items()
                        .iter().map(|obj| obj.downcast_ref::<model::Pod>().unwrap())
                        .for_each(|pod|
                    {
                        pod.delete(true, clone!(@weak obj => move |result| {
                            if let Err(e) = result {
                                utils::show_toast(
                                    &obj,
                                    // Translators: The "{}" is a placeholder for an error message.
                                    &gettext!("Error on deleting pod: {}", e)
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
