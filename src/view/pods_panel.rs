use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::ops::Deref;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gettextrs::ngettext;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::gio;
use gtk::glib;

use crate::config;
use crate::model;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

const ACTION_CREATE_POD: &str = "pods-panel.create-pod";
const ACTION_PRUNE_PODS: &str = "pods-panel.prune-pods";
const ACTION_ENTER_SELECTION_MODE: &str = "pods-panel.enter-selection-mode";

const ACTION_EXIT_SELECTION_MODE: &str = "pods-panel.exit-selection-mode";
const ACTION_SELECT_VISIBLE: &str = "pods-panel.select-visible";
const ACTION_SELECT_NONE: &str = "pods-panel.select-none";
const ACTION_KILL_SELECTION: &str = "pods-panel.kill-selection";
const ACTION_RESTART_SELECTION: &str = "pods-panel.restart-selection";
const ACTION_START_OR_RESUME_SELECTION: &str = "pods-panel.start-or-resume-selection";
const ACTION_STOP_SELECTION: &str = "pods-panel.stop-selection";
const ACTION_PAUSE_SELECTION: &str = "pods-panel.pause-selection";
const ACTION_DELETE_SELECTION: &str = "pods-panel.delete-selection";
const ACTION_TOGGLE_SORT_DIRECTION: &str = "pods-panel.toggle-sort-direction";
const ACTION_CHANGE_SORT_ATTRIBUTE: &str = "pods-panel.change-sort-attribute";
const ACTION_TOGGLE_SHOW_RUNNING_PODS_FIRST: &str = "pods-panel.toggle-show-running-pods-first";
const ACTION_SHOW_ALL_PODS: &str = "pods-panel.show-all-pods";

const ACTIONS_SELECTION: &[&str] = &[
    ACTION_KILL_SELECTION,
    ACTION_RESTART_SELECTION,
    ACTION_START_OR_RESUME_SELECTION,
    ACTION_STOP_SELECTION,
    ACTION_PAUSE_SELECTION,
    ACTION_DELETE_SELECTION,
];

#[derive(Debug)]
pub(crate) struct Settings(gio::Settings);
impl Default for Settings {
    fn default() -> Self {
        Self(gio::Settings::new(&format!(
            "{}.view.panels.pods",
            config::APP_ID
        )))
    }
}
impl Deref for Settings {
    type Target = gio::Settings;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "PodsPanelSortDirection")]
pub(crate) enum SortDirection {
    #[default]
    #[enum_value(nick = "asc")]
    Asc,
    #[enum_value(nick = "desc")]
    Desc,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "PodsPanelSortAttribute")]
pub(crate) enum SortAttribute {
    #[default]
    #[enum_value(nick = "name")]
    Name,
    #[enum_value(nick = "containers")]
    Containers,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PodsPanel)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/pods_panel.ui")]
    pub(crate) struct PodsPanel {
        pub(super) settings: Settings,
        pub(super) filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        pub(super) search_term: RefCell<String>,
        #[property(get, set = Self::set_pod_list, nullable)]
        pub(super) pod_list: glib::WeakRef<model::PodList>,
        #[property(get, set)]
        pub(super) collapsed: Cell<bool>,
        #[property(get, set, builder(SortDirection::default()))]
        pub(super) sort_direction: RefCell<SortDirection>,
        #[property(get, set, builder(SortAttribute::default()))]
        pub(super) sort_attribute: RefCell<SortAttribute>,
        #[property(get, set)]
        pub(super) show_running_pods_first: Cell<bool>,
        #[template_child]
        pub(super) create_pod_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) prune_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) view_options_split_button: TemplateChild<adw::SplitButton>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) toolbar_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub(super) header_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) create_pod_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) prune_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) view_options_split_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) selected_pods_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) filter_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) overhang_action_bar: TemplateChild<gtk::ActionBar>,
        #[template_child]
        pub(super) create_pod_button_bottom_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) prune_button_bottom_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) view_options_split_button_bottom_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodsPanel {
        const NAME: &'static str = "PdsPodsPanel";
        type Type = super::PodsPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_CREATE_POD,
            );
            klass.install_action(ACTION_CREATE_POD, None, |widget, _, _| {
                widget.create_pod();
            });

            klass.install_action(ACTION_PRUNE_PODS, None, |widget, _, _| {
                widget.prune_pods();
            });

            klass.install_action(ACTION_ENTER_SELECTION_MODE, None, |widget, _, _| {
                widget.enter_selection_mode();
            });
            klass.install_action(ACTION_EXIT_SELECTION_MODE, None, |widget, _, _| {
                widget.exit_selection_mode();
            });

            klass.install_action(ACTION_SELECT_VISIBLE, None, |widget, _, _| {
                widget.select_visible();
            });
            klass.install_action(ACTION_SELECT_NONE, None, |widget, _, _| {
                widget.select_none();
            });

            klass.install_action(ACTION_KILL_SELECTION, None, move |widget, _, _| {
                widget.stop_selection(true);
            });
            klass.install_action(ACTION_RESTART_SELECTION, None, |widget, _, _| {
                widget.restart_selection();
            });
            klass.install_action(
                ACTION_START_OR_RESUME_SELECTION,
                None,
                move |widget, _, _| {
                    widget.start_selection();
                },
            );
            klass.install_action(ACTION_STOP_SELECTION, None, |widget, _, _| {
                widget.stop_selection(false);
            });
            klass.install_action(ACTION_PAUSE_SELECTION, None, |widget, _, _| {
                widget.pause_selection();
            });
            klass.install_action(ACTION_DELETE_SELECTION, None, |widget, _, _| {
                widget.delete_selection();
            });

            klass.install_action(ACTION_TOGGLE_SORT_DIRECTION, None, |widget, _, _| {
                widget.toggle_sort_direction();
            });
            klass.install_property_action(ACTION_CHANGE_SORT_ATTRIBUTE, "sort-attribute");
            klass.install_property_action(
                ACTION_TOGGLE_SHOW_RUNNING_PODS_FIRST,
                "show-running-pods-first",
            );

            klass.install_action(ACTION_SHOW_ALL_PODS, None, |widget, _, _| {
                widget.show_all_pods();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodsPanel {
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
                .bind("sort-direction", obj, "sort-direction")
                .build();
            self.settings
                .bind("sort-attribute", obj, "sort-attribute")
                .build();
            self.settings
                .bind("show-running-first", obj, "show-running-pods-first")
                .build();

            let pod_list_expr = Self::Type::this_expression("pod-list");
            let pod_list_len_expr = pod_list_expr.chain_property::<model::PodList>("len");
            let selection_mode_expr =
                pod_list_expr.chain_property::<model::PodList>("selection-mode");
            let not_selection_mode_expr = selection_mode_expr.chain_closure::<bool>(closure!(
                |_: Self::Type, selection_mode: bool| { !selection_mode }
            ));
            let collapsed_expr = Self::Type::this_expression("collapsed");

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
            .bind(&self.main_stack.get(), "visible-child-name", Some(obj));

            selection_mode_expr
                .chain_closure::<String>(closure!(|_: Self::Type, selection_mode: bool| {
                    if !selection_mode { "main" } else { "selection" }
                }))
                .bind(&self.header_stack.get(), "visible-child-name", Some(obj));

            gtk::ClosureExpression::new::<Option<String>>(
                [
                    &pod_list_len_expr,
                    &pod_list_expr.chain_property::<model::PodList>("running"),
                ],
                closure!(|_: Self::Type, len: u32, running: u32| {
                    if len == 0 {
                        String::new()
                    } else if len == 1 {
                        if running == 1 {
                            gettext("1 pod, running")
                        } else {
                            gettext("1 pod, stopped")
                        }
                    } else {
                        ngettext!(
                            "{} pod total, {} running",
                            "{} pods total, {} running",
                            len,
                            len,
                            running,
                        )
                    }
                }),
            )
            .bind(&self.window_title.get(), "subtitle", Some(obj));

            Self::Type::this_expression("sort-direction")
                .chain_closure::<String>(closure!(|_: Self::Type, direction: SortDirection| {
                    match direction {
                        SortDirection::Asc => "view-sort-ascending-rtl-symbolic",
                        SortDirection::Desc => "view-sort-descending-rtl-symbolic",
                    }
                }))
                .bind(
                    &self.view_options_split_button.get(),
                    "icon-name",
                    Some(obj),
                );

            pod_list_expr
                .chain_property::<model::PodList>("num-selected")
                .chain_closure::<String>(closure!(|_: Self::Type, selected: u32| ngettext!(
                    "{} Selected Pod",
                    "{} Selected Pods",
                    selected,
                    selected
                )))
                .bind(&self.selected_pods_button.get(), "label", Some(obj));

            not_selection_mode_expr.bind(&self.search_bar.get(), "visible", Some(obj));

            gtk::ClosureExpression::new::<bool>(
                [
                    collapsed_expr.upcast_ref(),
                    not_selection_mode_expr.upcast_ref(),
                ],
                closure!(|_: Self::Type, collapsed: bool, not_selection_mode: bool| {
                    collapsed && not_selection_mode
                }),
            )
            .bind(&self.overhang_action_bar.get(), "revealed", Some(obj));

            gtk::ClosureExpression::new::<bool>(
                [
                    collapsed_expr.upcast_ref(),
                    selection_mode_expr.upcast_ref(),
                ],
                closure!(|_: Self::Type, collapsed: bool, selection_mode: bool| {
                    collapsed || selection_mode
                }),
            )
            .bind(&self.toolbar_view.get(), "reveal-bottom-bars", Some(obj));

            let filter = gtk::CustomFilter::new(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                false,
                move |item| {
                    let term = &*obj.imp().search_term.borrow();
                    item.downcast_ref::<model::Pod>()
                        .unwrap()
                        .name()
                        .to_lowercase()
                        .contains(term)
                }
            ));

            let sorter = gtk::CustomSorter::new(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                gtk::Ordering::Equal,
                move |item1, item2| {
                    let pod1 = item1.downcast_ref::<model::Pod>().unwrap();
                    let pod2 = item2.downcast_ref::<model::Pod>().unwrap();

                    if obj.show_running_pods_first() {
                        match pod2.status().cmp(&pod1.status()) {
                            std::cmp::Ordering::Equal => obj.imp().ordering(pod1, pod2),
                            other => other,
                        }
                    } else {
                        obj.imp().ordering(pod1, pod2)
                    }
                    .into()
                }
            ));

            self.filter.set(filter.upcast()).unwrap();
            self.sorter.set(sorter.upcast()).unwrap();
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for PodsPanel {}

    #[gtk::template_callbacks]
    impl PodsPanel {
        fn ordering(&self, pod1: &model::Pod, pod2: &model::Pod) -> std::cmp::Ordering {
            let obj = self.obj();
            let ordering = match obj.sort_attribute() {
                SortAttribute::Name => pod1.name().to_lowercase().cmp(&pod2.name().to_lowercase()),
                SortAttribute::Containers => pod1.num_containers().cmp(&pod2.num_containers()),
            };

            match obj.sort_direction() {
                SortDirection::Asc => ordering,
                SortDirection::Desc => ordering.reverse(),
            }
        }

        #[template_callback]
        fn on_notify_collapsed(&self) {
            if self.obj().collapsed() {
                self.create_pod_button_top_bin.set_child(gtk::Widget::NONE);
                self.prune_button_top_bin.set_child(gtk::Widget::NONE);
                self.view_options_split_button_top_bin
                    .set_child(gtk::Widget::NONE);

                self.create_pod_button_bottom_bin
                    .set_child(Some(&self.create_pod_button.get()));
                self.prune_button_bottom_bin
                    .set_child(Some(&self.prune_button.get()));
                self.view_options_split_button_bottom_bin
                    .set_child(Some(&self.view_options_split_button.get()));
            } else {
                self.create_pod_button_bottom_bin
                    .set_child(gtk::Widget::NONE);
                self.prune_button_bottom_bin.set_child(gtk::Widget::NONE);
                self.view_options_split_button_bottom_bin
                    .set_child(gtk::Widget::NONE);

                self.create_pod_button_top_bin
                    .set_child(Some(&self.create_pod_button.get()));
                self.prune_button_top_bin
                    .set_child(Some(&self.prune_button.get()));
                self.view_options_split_button_top_bin
                    .set_child(Some(&self.view_options_split_button.get()));
            }
        }

        #[template_callback]
        fn on_notify_sort_attribute(&self) {
            self.update_sorter();
        }

        #[template_callback]
        fn on_notify_show_running_pods_first(&self) {
            self.update_sorter();
        }

        #[template_callback]
        fn on_notify_search_mode_enabled(&self) {
            if self.search_bar.is_search_mode() {
                self.search_entry.grab_focus();
            }
        }

        #[template_callback]
        fn on_search_changed(&self) {
            let term = self.search_entry.text().trim().to_lowercase();

            let filter_change = if self.search_term.borrow().contains(&term) {
                gtk::FilterChange::LessStrict
            } else {
                gtk::FilterChange::MoreStrict
            };

            self.search_term.replace(term);
            self.update_filter(filter_change);
        }

        pub(crate) fn set_pod_list(&self, value: &model::PodList) {
            let obj = &*self.obj();
            if obj.pod_list().as_ref() == Some(value) {
                return;
            }

            value.connect_containers_in_pod_changed(clone!(
                #[weak]
                obj,
                move |_, _| {
                    glib::timeout_add_seconds_local_once(
                        1,
                        clone!(
                            #[weak]
                            obj,
                            move || if obj.sort_attribute() == SortAttribute::Containers {
                                obj.imp().update_sorter();
                            }
                        ),
                    );
                }
            ));

            let model = gtk::SortListModel::new(
                Some(gtk::FilterListModel::new(
                    Some(value.to_owned()),
                    self.filter.get().cloned(),
                )),
                self.sorter.get().cloned(),
            );

            self.list_box.bind_model(Some(&model), |item| {
                view::PodRow::from(item.downcast_ref().unwrap()).upcast()
            });

            self.set_filter_stack_visible_child(value, &model);
            model.connect_items_changed(clone!(
                #[weak]
                obj,
                #[weak]
                value,
                move |model, _, removed, _| {
                    obj.imp().set_filter_stack_visible_child(&value, model);

                    if removed > 0 {
                        obj.deselect_hidden_pods(model.upcast_ref());
                    }
                }
            ));
            value.connect_initialized_notify(clone!(
                #[weak]
                obj,
                #[weak]
                model,
                move |value| obj.imp().set_filter_stack_visible_child(value, &model)
            ));

            ACTIONS_SELECTION
                .iter()
                .for_each(|action_name| obj.action_set_enabled(action_name, false));
            value.connect_notify_local(
                Some("num-selected"),
                clone!(
                    #[weak]
                    obj,
                    move |list, _| {
                        ACTIONS_SELECTION.iter().for_each(|action_name| {
                            obj.action_set_enabled(action_name, list.num_selected() > 0);
                        });
                    }
                ),
            );

            self.pod_list.set(Some(value));
        }

        fn set_filter_stack_visible_child(
            &self,
            pod_list: &model::PodList,
            model: &impl IsA<gio::ListModel>,
        ) {
            self.filter_stack.set_visible_child_name(
                if model.n_items() > 0 || !pod_list.initialized() {
                    "list"
                } else {
                    "empty"
                },
            );
        }

        fn update_filter(&self, filter_change: gtk::FilterChange) {
            if let Some(filter) = self.filter.get() {
                filter.changed(filter_change);
            }
        }

        pub(super) fn update_sorter(&self) {
            self.sorter
                .get()
                .unwrap()
                .changed(gtk::SorterChange::Different);
        }
    }
}

glib::wrapper! {
    pub(crate) struct PodsPanel(ObjectSubclass<imp::PodsPanel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for PodsPanel {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl PodsPanel {
    pub(crate) fn toggle_sort_direction(&self) {
        self.set_sort_direction(match self.sort_direction() {
            SortDirection::Asc => SortDirection::Desc,
            SortDirection::Desc => SortDirection::Asc,
        });
        self.imp().update_sorter();
    }

    pub(crate) fn show_all_pods(&self) {
        self.set_search_mode(false);
    }

    pub(crate) fn set_search_mode(&self, value: bool) {
        self.imp().search_bar.set_search_mode(value);
    }

    pub(crate) fn toggle_search_mode(&self) {
        self.set_search_mode(!self.imp().search_bar.is_search_mode());
    }

    pub(crate) fn create_pod(&self) {
        if let Some(client) = self.pod_list().as_ref().and_then(model::PodList::client) {
            utils::Dialog::new(self, &view::PodCreationPage::from(&client)).present();
        }
    }

    pub(crate) fn prune_pods(&self) {
        if let Some(client) = self.pod_list().and_then(|pod_list| pod_list.client()) {
            utils::Dialog::new(self, &view::PodsPrunePage::from(&client)).present();
        }
    }

    pub(crate) fn enter_selection_mode(&self) {
        if let Some(list) = self.pod_list().filter(|list| list.len() > 0) {
            list.select_none();
            list.set_selection_mode(true);
        }
    }

    pub(crate) fn exit_selection_mode(&self) {
        if let Some(list) = self.pod_list() {
            list.set_selection_mode(false);
        }
    }

    pub(crate) fn select_visible(&self) {
        (0..)
            .map(|pos| self.imp().list_box.row_at_index(pos))
            .take_while(Option::is_some)
            .flatten()
            .for_each(|row| {
                row.downcast_ref::<view::PodRow>()
                    .unwrap()
                    .pod()
                    .unwrap()
                    .set_selected(row.is_visible());
            });
    }

    pub(crate) fn select_none(&self) {
        if let Some(list) = self.pod_list().filter(|list| list.is_selection_mode()) {
            list.select_none();
        }
    }

    pub(crate) fn start_selection(&self) {
        if let Some(list) = self.pod_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Pod>().unwrap())
                .for_each(|pod| match pod.status() {
                    model::PodStatus::Paused => {
                        pod.resume(clone!(
                            #[weak(rename_to = obj)]
                            self,
                            move |result| {
                                if let Err(e) = result {
                                    utils::show_error_toast(
                                        &obj,
                                        &gettext("Error on resuming pod"),
                                        &e.to_string(),
                                    );
                                }
                            }
                        ));
                    }
                    other if other != model::PodStatus::Running => {
                        pod.start(clone!(
                            #[weak(rename_to = obj)]
                            self,
                            move |result| {
                                if let Err(e) = result {
                                    utils::show_error_toast(
                                        &obj,
                                        &gettext("Error on starting pod"),
                                        &e.to_string(),
                                    );
                                }
                            }
                        ));
                    }
                    _ => (),
                });
            list.set_selection_mode(false);
        }
    }

    pub(crate) fn stop_selection(&self, force: bool) {
        if let Some(list) = self.pod_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Pod>().unwrap())
                .filter(|pod| matches!(pod.status(), model::PodStatus::Running))
                .for_each(|pod| {
                    pod.stop(
                        force,
                        clone!(
                            #[weak(rename_to = obj)]
                            self,
                            move |result| {
                                if let Err(e) = result {
                                    utils::show_error_toast(
                                        &obj,
                                        &if force {
                                            gettext("Error on killing pod")
                                        } else {
                                            gettext("Error on stopping pod")
                                        },
                                        &e.to_string(),
                                    );
                                }
                            }
                        ),
                    );
                });
            list.set_selection_mode(false);
        }
    }

    pub(crate) fn pause_selection(&self) {
        if let Some(list) = self.pod_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Pod>().unwrap())
                .filter(|pod| matches!(pod.status(), model::PodStatus::Running))
                .for_each(|pod| {
                    pod.pause(clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |result| {
                            if let Err(e) = result {
                                utils::show_error_toast(
                                    &obj,
                                    &gettext("Error on stopping pod"),
                                    &e.to_string(),
                                );
                            }
                        }
                    ));
                });
            list.set_selection_mode(false);
        }
    }

    pub(crate) fn restart_selection(&self) {
        if let Some(list) = self.pod_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Pod>().unwrap())
                .filter(|pod| matches!(pod.status(), model::PodStatus::Running))
                .for_each(|pod| {
                    pod.restart(
                        false,
                        clone!(
                            #[weak(rename_to = obj)]
                            self,
                            move |result| {
                                if let Err(e) = result {
                                    utils::show_error_toast(
                                        &obj,
                                        &gettext("Error on restarting pod"),
                                        &e.to_string(),
                                    );
                                }
                            }
                        ),
                    );
                });
            list.set_selection_mode(false);
        }
    }

    pub(crate) fn delete_selection(&self) {
        if self.pod_list().map(|list| list.num_selected()).unwrap_or(0) == 0 {
            return;
        }

        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Confirm Forced Deletion of Multiple Pods"))
            .body_use_markup(true)
            .body(gettext("All associated containers will also be removed!"))
            .build();

        dialog.add_responses(&[
            ("cancel", &gettext("_Cancel")),
            ("delete", &gettext("_Delete")),
        ]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

        dialog.connect_response(
            None,
            clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, response| if response == "delete"
                    && let Some(list) = obj.pod_list()
                {
                    list.selected_items()
                        .iter()
                        .map(|obj| obj.downcast_ref::<model::Pod>().unwrap())
                        .for_each(|pod| {
                            pod.delete(
                                true,
                                clone!(
                                    #[weak]
                                    obj,
                                    move |result| {
                                        if let Err(e) = result {
                                            utils::show_error_toast(
                                                &obj,
                                                &gettext("Error on deleting pod"),
                                                &e.to_string(),
                                            );
                                        }
                                    }
                                ),
                            );
                        });
                    list.set_selection_mode(false);
                }
            ),
        );

        dialog.present(Some(self));
    }

    fn deselect_hidden_pods(&self, model: &gio::ListModel) {
        let visible_pods = model
            .iter::<glib::Object>()
            .map(Result::unwrap)
            .map(|item| item.downcast::<model::Pod>().unwrap())
            .collect::<Vec<_>>();

        self.pod_list()
            .unwrap()
            .iter::<model::Pod>()
            .map(Result::unwrap)
            .filter(model::Pod::selected)
            .for_each(|pod| {
                if !visible_pods.contains(&pod) {
                    pod.set_selected(false);
                }
            });
    }
}
