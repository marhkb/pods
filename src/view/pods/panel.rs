use adw::prelude::MessageDialogExtManual;
use adw::traits::MessageDialogExt;
use gettextrs::gettext;
use gettextrs::ngettext;
use glib::clone;
use glib::closure;
use glib::subclass::Signal;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy as SyncLazy;
use once_cell::unsync::OnceCell as UnsyncOnceCell;

use crate::model;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

const ACTION_CREATE_POD: &str = "pods-panel.create-pod";
const ACTION_START_OR_RESUME_SELECTION: &str = "pods-panel.start-or-resume-selection";
const ACTION_STOP_SELECTION: &str = "pods-panel.stop-selection";
const ACTION_PAUSE_SELECTION: &str = "pods-panel.pause-selection";
const ACTION_RESTART_SELECTION: &str = "pods-panel.restart-selection";
const ACTION_DELETE_SELECTION: &str = "pods-panel.delete-selection";

const ACTIONS_SELECTION: &[&str] = &[
    ACTION_START_OR_RESUME_SELECTION,
    ACTION_STOP_SELECTION,
    ACTION_PAUSE_SELECTION,
    ACTION_RESTART_SELECTION,
    ACTION_DELETE_SELECTION,
];

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::Panel)]
    #[template(resource = "/com/github/marhkb/Pods/ui/pods/panel.ui")]
    pub(crate) struct Panel {
        pub(super) settings: utils::PodsSettings,
        pub(super) properties_filter: UnsyncOnceCell<gtk::Filter>,
        pub(super) sorter: UnsyncOnceCell<gtk::Sorter>,
        #[property(get, set = Self::set_pod_list, explicit_notify, nullable)]
        pub(super) pod_list: glib::WeakRef<model::PodList>,
        #[template_child]
        pub(super) create_pod_row: TemplateChild<gtk::ListBoxRow>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) pods_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) header_suffix_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) show_only_running_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) create_pod_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Panel {
        const NAME: &'static str = "PdsPodsPanel";
        type Type = super::Panel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_CREATE_POD,
                None,
            );
            klass.install_action(ACTION_CREATE_POD, None, |widget, _, _| {
                widget.create_pod();
            });

            klass.install_action(
                ACTION_START_OR_RESUME_SELECTION,
                None,
                move |widget, _, _| {
                    widget.start_selection();
                },
            );
            klass.install_action(ACTION_STOP_SELECTION, None, |widget, _, _| {
                widget.stop_selection();
            });
            klass.install_action(ACTION_PAUSE_SELECTION, None, |widget, _, _| {
                widget.pause_selection();
            });
            klass.install_action(ACTION_RESTART_SELECTION, None, |widget, _, _| {
                widget.restart_selection();
            });
            klass.install_action(ACTION_DELETE_SELECTION, None, |widget, _, _| {
                widget.delete_selection();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Panel {
        fn signals() -> &'static [Signal] {
            static SIGNALS: SyncLazy<Vec<Signal>> =
                SyncLazy::new(|| vec![Signal::builder("exit-selection-mode").build()]);
            SIGNALS.as_ref()
        }

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

            self.settings
                .bind(
                    "show-only-running-pods",
                    &*self.show_only_running_switch,
                    "active",
                )
                .build();

            let pod_list_expr = Self::Type::this_expression("pod-list");
            let pod_list_len_expr = pod_list_expr.chain_property::<model::PodList>("len");
            let is_selection_mode_expr = pod_list_expr
                .chain_property::<model::PodList>("selection-mode")
                .chain_closure::<bool>(closure!(|_: Self::Type, selection_mode: bool| {
                    !selection_mode
                }));

            pod_list_len_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, len: u32| len > 0))
                .bind(&*self.header_suffix_box, "visible", Some(obj));

            is_selection_mode_expr.bind(&*self.create_pod_button, "visible", Some(obj));
            is_selection_mode_expr.bind(&*self.create_pod_row, "visible", Some(obj));

            pod_list_len_expr.watch(
                Some(obj),
                clone!(@weak obj => move || {
                    let list = obj.pod_list().unwrap();
                    if list.is_selection_mode() && list.len() == 0 {
                        list.set_selection_mode(false);
                        obj.emit_by_name::<()>("exit-selection-mode", &[]);
                    }
                }),
            );

            gtk::ClosureExpression::new::<Option<String>>(
                [
                    &pod_list_len_expr,
                    &pod_list_expr.chain_property::<model::PodList>("listing"),
                    &pod_list_expr.chain_property::<model::PodList>("initialized"),
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
                            Some("pods")
                        }
                    }
                ),
            )
            .bind(&*self.main_stack, "visible-child-name", Some(obj));

            gtk::ClosureExpression::new::<Option<String>>(
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

            self.show_only_running_switch.connect_active_notify(
                clone!(@weak obj => move |switch| {
                    obj.update_properties_filter(
                        if switch.is_active() {
                            gtk::FilterChange::MoreStrict
                        } else {
                            gtk::FilterChange::LessStrict
                        }
                    );
                }),
            );
        }

        fn dispose(&self) {
            self.main_stack.unparent();
        }
    }

    impl WidgetImpl for Panel {}

    impl Panel {
        pub(crate) fn set_pod_list(&self, value: &model::PodList) {
            let obj = &*self.obj();
            if obj.pod_list().as_ref() == Some(value) {
                return;
            }

            value.connect_notify_local(
                Some("running"),
                clone!(@weak obj => move |_ ,_| {
                    obj.update_properties_filter(gtk::FilterChange::Different);
                }),
            );

            let model = gtk::SortListModel::new(
                Some(gtk::FilterListModel::new(
                    Some(value.to_owned()),
                    self.properties_filter.get().cloned(),
                )),
                self.sorter.get().cloned(),
            );

            self.list_box.bind_model(Some(&model), |item| {
                view::PodRow::from(item.downcast_ref().unwrap()).upcast()
            });
            self.list_box.append(&*self.create_pod_row);

            ACTIONS_SELECTION
                .iter()
                .for_each(|action_name| obj.action_set_enabled(action_name, false));
            value.connect_notify_local(
                Some("num-selected"),
                clone!(@weak obj => move |list, _| {
                    ACTIONS_SELECTION
                        .iter()
                        .for_each(|action_name| {
                            obj.action_set_enabled(action_name, list.num_selected() > 0);
                    });
                }),
            );

            self.pod_list.set(Some(value));
            obj.notify("pod-list");
        }
    }
}

glib::wrapper! {
    pub(crate) struct Panel(ObjectSubclass<imp::Panel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Panel {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl Panel {
    pub(crate) fn action_create_pod() -> &'static str {
        ACTION_CREATE_POD
    }

    pub(crate) fn update_properties_filter(&self, filter_change: gtk::FilterChange) {
        self.imp()
            .properties_filter
            .get()
            .unwrap()
            .changed(filter_change);
    }

    fn create_pod(&self) {
        if let Some(client) = self.pod_list().as_ref().and_then(model::PodList::client) {
            utils::show_dialog(
                self.upcast_ref(),
                view::PodCreationPage::from(&client).upcast_ref(),
            );
        }
    }

    fn start_selection(&self) {
        if let Some(list) = self.pod_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Pod>().unwrap())
                .for_each(|pod| match pod.status() {
                    model::PodStatus::Paused => {
                        pod.resume(clone!(@weak  self as obj => move |result| {
                            if let Err(e) = result {
                                utils::show_error_toast(
                                    obj.upcast_ref(),
                                    &gettext("Error on resuming pod"),
                                    &e.to_string(),
                                );
                            }
                        }));
                    }
                    other if other != model::PodStatus::Running => {
                        pod.start(clone!(@weak  self as obj => move |result| {
                            if let Err(e) = result {
                                utils::show_error_toast(
                                    obj.upcast_ref(),
                                    &gettext("Error on starting pod"),
                                    &e.to_string(),
                                );
                            }
                        }));
                    }
                    _ => (),
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
                                utils::show_error_toast(
                                    obj.upcast_ref(),
                                    &gettext("Error on stopping pod"),
                                    &e.to_string(),
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
                            utils::show_error_toast(
                                obj.upcast_ref(),
                                &gettext("Error on stopping pod"),
                                &e.to_string(),
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
                                utils::show_error_toast(
                                    obj.upcast_ref(),
                                    &gettext("Error on restarting pod"),
                                    &e.to_string(),
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
            .heading(gettext("Confirm Forced Deletion of Multiple Pods"))
            .body_use_markup(true)
            .body(gettext("All associated containers will also be removed!"))
            .modal(true)
            .transient_for(&utils::root(self.upcast_ref()))
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
                                utils::show_error_toast(
                                    obj.upcast_ref(),
                                    &gettext("Error on deleting pod"),
                                    &e.to_string(),
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
