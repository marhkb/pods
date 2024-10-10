use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gettextrs::ngettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::model::AbstractContainerListExt;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

const ACTION_CREATE_CONTAINER: &str = "containers-panel.create-container";
const ACTION_PRUNE_UNUSED_CONTAINERS: &str = "containers-panel.prune-unused-containers";
const ACTION_ENTER_SELECTION_MODE: &str = "containers-panel.enter-selection-mode";
const ACTION_EXIT_SELECTION_MODE: &str = "containers-panel.exit-selection-mode";
const ACTION_SELECT_VISIBLE: &str = "containers-panel.select-visible";
const ACTION_SELECT_NONE: &str = "containers-panel.select-none";
const ACTION_KILL_SELECTION: &str = "containers-panel.kill-selection";
const ACTION_RESTART_SELECTION: &str = "containers-panel.restart-selection";
const ACTION_START_OR_RESUME_SELECTION: &str = "containers-panel.start-or-resume-selection";
const ACTION_STOP_SELECTION: &str = "containers-panel.stop-selection";
const ACTION_PAUSE_SELECTION: &str = "containers-panel.pause-selection";
const ACTION_DELETE_SELECTION: &str = "containers-panel.delete-selection";
const ACTION_TOGGLE_SHOW_ONLY_RUNNING_CONTAINERS: &str =
    "containers-panel.toggle-show-only-running-containers";
const ACTION_SHOW_ALL_CONTAINERS: &str = "containers-panel.show-all-containers";

const ACTIONS_SELECTION: &[&str] = &[
    ACTION_KILL_SELECTION,
    ACTION_RESTART_SELECTION,
    ACTION_START_OR_RESUME_SELECTION,
    ACTION_STOP_SELECTION,
    ACTION_PAUSE_SELECTION,
    ACTION_DELETE_SELECTION,
];

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainersPanel)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/containers_panel.ui")]
    pub(crate) struct ContainersPanel {
        pub(super) settings: utils::PodsSettings,
        pub(super) filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        pub(super) search_term: RefCell<String>,
        #[property(get, set = Self::set_container_list, nullable)]
        pub(super) container_list: glib::WeakRef<model::ContainerList>,
        #[property(get, set)]
        pub(super) show_only_running_containers: Cell<bool>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) header_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) selected_containers_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) filter_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) flow_box: TemplateChild<gtk::FlowBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainersPanel {
        const NAME: &'static str = "PdsContainersPanel";
        type Type = super::ContainersPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_CREATE_CONTAINER,
            );
            klass.install_action(ACTION_CREATE_CONTAINER, None, move |widget, _, _| {
                widget.create_container();
            });

            klass.install_action(ACTION_PRUNE_UNUSED_CONTAINERS, None, |widget, _, _| {
                widget.show_prune_page();
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
                    widget.start_or_resume_selection();
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

            klass.install_property_action(
                ACTION_TOGGLE_SHOW_ONLY_RUNNING_CONTAINERS,
                "show-only-running-containers",
            );

            klass.install_action(ACTION_SHOW_ALL_CONTAINERS, None, |widget, _, _| {
                widget.show_all_containers();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainersPanel {
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
                    "show-only-running-containers",
                    obj,
                    "show-only-running-containers",
                )
                .build();

            let container_list_expr = Self::Type::this_expression("container-list");
            let container_list_containers_expr =
                container_list_expr.chain_property::<model::ContainerList>("containers");
            let selection_mode_expr =
                container_list_expr.chain_property::<model::ContainerList>("selection-mode");
            let not_selection_mode_expr = selection_mode_expr.chain_closure::<bool>(closure!(
                |_: Self::Type, selection_mode: bool| { !selection_mode }
            ));

            gtk::ClosureExpression::new::<Option<String>>(
                [
                    &container_list_containers_expr,
                    &container_list_expr.chain_property::<model::ContainerList>("listing"),
                    &container_list_expr.chain_property::<model::ContainerList>("initialized"),
                ],
                closure!(
                    |_: Self::Type, containers: u32, listing: bool, initialized: bool| {
                        if containers == 0 {
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
            .bind(&self.main_stack.get(), "visible-child-name", Some(obj));

            selection_mode_expr
                .chain_closure::<String>(closure!(|_: Self::Type, selection_mode: bool| {
                    if !selection_mode {
                        "main"
                    } else {
                        "selection"
                    }
                }))
                .bind(&self.header_stack.get(), "visible-child-name", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [
                    &container_list_containers_expr,
                    &container_list_expr.chain_property::<model::AbstractContainerList>("running"),
                ],
                closure!(|_: Self::Type, containers: u32, running: u32| {
                    if containers == 0 {
                        String::new()
                    } else if containers == 1 {
                        if running == 1 {
                            gettext("1 container, running")
                        } else {
                            gettext("1 container, stopped")
                        }
                    } else {
                        ngettext!(
                            "{} container total, {} running",
                            "{} containers total, {} running",
                            containers,
                            containers,
                            running,
                        )
                    }
                }),
            )
            .bind(&self.window_title.get(), "subtitle", Some(obj));

            container_list_expr
                .chain_property::<model::ContainerList>("num-selected")
                .chain_closure::<String>(closure!(|_: Self::Type, selected: u32| ngettext!(
                    "{} Selected Container",
                    "{} Selected Containers",
                    selected,
                    selected
                )))
                .bind(&self.selected_containers_button.get(), "label", Some(obj));

            not_selection_mode_expr.bind(&self.search_bar.get(), "visible", Some(obj));

            let search_filter = gtk::CustomFilter::new(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                false,
                move |item| {
                    let term = &*obj.imp().search_term.borrow();

                    if term.is_empty() {
                        true
                    } else {
                        let container = item.downcast_ref::<model::Container>().unwrap();
                        container.name().to_lowercase().contains(term)
                            || container.id().contains(term)
                            || container
                                .image_name()
                                .map(|image_name| image_name.to_lowercase().contains(term))
                                .unwrap_or(false)
                            || container.image_id().contains(term)
                    }
                }
            ));

            let state_filter = gtk::AnyFilter::new();
            state_filter.append(gtk::CustomFilter::new(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                false,
                move |_| !obj.show_only_running_containers()
            )));
            state_filter.append(gtk::BoolFilter::new(Some(
                model::Container::this_expression("status").chain_closure::<bool>(closure!(
                    |_: model::Container, status: model::ContainerStatus| status
                        == model::ContainerStatus::Running
                )),
            )));

            let filter = gtk::EveryFilter::new();
            filter.append(
                gtk::BoolFilter::builder()
                    .expression(model::Container::this_expression("is-infra"))
                    .invert(true)
                    .build(),
            );
            filter.append(search_filter);
            filter.append(state_filter);

            let sorter = gtk::CustomSorter::new(|item1, item2| {
                item1
                    .downcast_ref::<model::Container>()
                    .unwrap()
                    .name()
                    .to_lowercase()
                    .cmp(
                        &item2
                            .downcast_ref::<model::Container>()
                            .unwrap()
                            .name()
                            .to_lowercase(),
                    )
                    .into()
            });

            self.filter.set(filter.upcast()).unwrap();
            self.sorter.set(sorter.upcast()).unwrap();
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ContainersPanel {}

    #[gtk::template_callbacks]
    impl ContainersPanel {
        #[template_callback]
        fn on_notify_show_only_running_containers(&self) {
            self.update_filter(if self.obj().show_only_running_containers() {
                gtk::FilterChange::MoreStrict
            } else {
                gtk::FilterChange::LessStrict
            });
        }

        #[template_callback]
        fn on_notify_search_mode_enabled(&self) {
            if self.search_bar.is_search_mode() {
                self.search_entry.grab_focus();
            } else {
                self.search_entry.set_text("");
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

        pub(super) fn set_container_list(&self, value: Option<&model::ContainerList>) {
            let obj = &*self.obj();
            if obj.container_list().as_ref() == value {
                return;
            }

            ACTIONS_SELECTION
                .iter()
                .for_each(|action_name| obj.action_set_enabled(action_name, false));

            if let Some(container_list) = value {
                container_list.connect_notify_local(
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

                container_list.connect_notify_local(
                    Some("running"),
                    clone!(
                        #[weak]
                        obj,
                        move |_, _| obj.imp().update_filter(gtk::FilterChange::Different)
                    ),
                );

                container_list.connect_container_name_changed(clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.imp().update_filter(gtk::FilterChange::Different);
                        glib::timeout_add_seconds_local_once(
                            1,
                            clone!(
                                #[weak]
                                obj,
                                move || obj.imp().update_sorter()
                            ),
                        );
                    }
                ));

                let model = gtk::SortListModel::new(
                    Some(gtk::FilterListModel::new(
                        Some(container_list.to_owned()),
                        self.filter.get().cloned(),
                    )),
                    self.sorter.get().cloned(),
                );

                self.flow_box.bind_model(Some(&model), |item| {
                    gtk::FlowBoxChild::builder()
                        .focusable(false)
                        .child(&view::ContainerCard::from(item.downcast_ref().unwrap()))
                        .build()
                        .upcast()
                });

                self.filter_stack
                    .set_visible_child_name(if model.n_items() > 0 { "list" } else { "empty" });
                model.connect_items_changed(clone!(
                    #[weak]
                    obj,
                    move |model, _, removed, _| {
                        obj.imp()
                            .filter_stack
                            .set_visible_child_name(if model.n_items() > 0 {
                                "list"
                            } else {
                                "empty"
                            });

                        if removed > 0 {
                            obj.deselect_hidden_containers(model.upcast_ref());
                        }
                    }
                ));
            }

            self.container_list.set(value);
        }

        fn update_filter(&self, filter_change: gtk::FilterChange) {
            if let Some(filter) = self.filter.get() {
                filter.changed(filter_change);
            }
        }

        fn update_sorter(&self) {
            self.sorter
                .get()
                .unwrap()
                .changed(gtk::SorterChange::Different);
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainersPanel(ObjectSubclass<imp::ContainersPanel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ContainersPanel {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl ContainersPanel {
    fn client(&self) -> Option<model::Client> {
        self.container_list()
            .as_ref()
            .and_then(model::ContainerList::client)
    }

    pub(crate) fn show_all_containers(&self) {
        self.set_show_only_running_containers(false);
        self.set_search_mode(false);
    }

    pub(crate) fn set_search_mode(&self, value: bool) {
        self.imp().search_bar.set_search_mode(value);
    }

    pub(crate) fn toggle_search_mode(&self) {
        self.set_search_mode(!self.imp().search_bar.is_search_mode());
    }

    pub(crate) fn create_container(&self) {
        if let Some(client) = self
            .container_list()
            .as_ref()
            .and_then(model::ContainerList::client)
        {
            utils::Dialog::new(self, &view::ContainerCreationPage::from(&client)).present();
        }
    }

    pub(crate) fn show_prune_page(&self) {
        if let Some(client) = self.client() {
            utils::Dialog::new(self, &view::ContainersPrunePage::from(&client))
                .follows_content_size(true)
                .present();
        }
    }

    pub(crate) fn enter_selection_mode(&self) {
        if let Some(list) = self.container_list().filter(|list| list.len() > 0) {
            list.select_none();
            list.set_selection_mode(true);
        }
    }

    pub(crate) fn exit_selection_mode(&self) {
        if let Some(list) = self.container_list() {
            list.set_selection_mode(false);
        }
    }

    pub(crate) fn select_visible(&self) {
        (0..)
            .map(|pos| self.imp().flow_box.child_at_index(pos))
            .take_while(Option::is_some)
            .flatten()
            .for_each(|row| {
                row.child()
                    .unwrap()
                    .downcast_ref::<view::ContainerCard>()
                    .unwrap()
                    .container()
                    .unwrap()
                    .set_selected(row.is_visible());
            });
    }

    pub(crate) fn select_none(&self) {
        if let Some(list) = self
            .container_list()
            .filter(|list| list.is_selection_mode())
        {
            list.select_none();
        }
    }

    pub(crate) fn stop_selection(&self, force: bool) {
        if let Some(list) = self.container_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Container>().unwrap())
                .filter(|container| matches!(container.status(), model::ContainerStatus::Running))
                .for_each(|container| {
                    container.stop(
                        force,
                        clone!(
                            #[weak(rename_to = obj)]
                            self,
                            move |result| {
                                if let Err(e) = result {
                                    utils::show_error_toast(
                                        &obj,
                                        &if force {
                                            gettext("Error on killing container")
                                        } else {
                                            gettext("Error on stopping container")
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

    pub(crate) fn restart_selection(&self) {
        if let Some(list) = self.container_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Container>().unwrap())
                .filter(|container| matches!(container.status(), model::ContainerStatus::Running))
                .for_each(|container| {
                    container.restart(
                        false,
                        clone!(
                            #[weak(rename_to = obj)]
                            self,
                            move |result| {
                                if let Err(e) = result {
                                    utils::show_error_toast(
                                        &obj,
                                        &gettext("Error on restarting container"),
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

    pub(crate) fn start_or_resume_selection(&self) {
        if let Some(list) = self.container_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Container>().unwrap())
                .for_each(|container| match container.status() {
                    model::ContainerStatus::Paused => {
                        container.resume(clone!(
                            #[weak(rename_to = obj)]
                            self,
                            move |result| {
                                if let Err(e) = result {
                                    utils::show_error_toast(
                                        &obj,
                                        &gettext("Error on resuming container"),
                                        &e.to_string(),
                                    );
                                }
                            }
                        ));
                    }
                    other if other != model::ContainerStatus::Running => {
                        container.start(clone!(
                            #[weak(rename_to = obj)]
                            self,
                            move |result| {
                                if let Err(e) = result {
                                    utils::show_error_toast(
                                        &obj,
                                        &gettext("Error on starting container"),
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

    pub(crate) fn pause_selection(&self) {
        if let Some(list) = self.container_list() {
            list.selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Container>().unwrap())
                .filter(|container| matches!(container.status(), model::ContainerStatus::Running))
                .for_each(|container| {
                    container.pause(clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |result| {
                            if let Err(e) = result {
                                utils::show_error_toast(
                                    &obj,
                                    &gettext("Error on pausing container"),
                                    &e.to_string(),
                                );
                            }
                        }
                    ));
                });
            list.set_selection_mode(false);
        }
    }

    pub(crate) fn delete_selection(&self) {
        if self
            .container_list()
            .map(|list| list.num_selected())
            .unwrap_or(0)
            == 0
        {
            return;
        }

        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Confirm Forced Deletion of Multiple Containers"))
            .body_use_markup(true)
            .body(gettext(
                "All the data created inside the containers will be lost and running containers will be stopped!",
            ))
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
                move |_, response| if response == "delete" {
                    if let Some(list) = obj.container_list() {
                        list.selected_items()
                            .iter()
                            .map(|obj| obj.downcast_ref::<model::Container>().unwrap())
                            .for_each(|container| {
                                container.delete(
                                    true,
                                    clone!(
                                        #[weak]
                                        obj,
                                        move |result| {
                                            if let Err(e) = result {
                                                utils::show_error_toast(
                                                    &obj,
                                                    &gettext("Error on deleting container"),
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
            ),
        );

        dialog.present(Some(self));
    }

    fn deselect_hidden_containers(&self, model: &gio::ListModel) {
        let visible_containers = model
            .iter::<glib::Object>()
            .map(Result::unwrap)
            .map(|item| item.downcast::<model::Container>().unwrap())
            .collect::<Vec<_>>();

        self.container_list()
            .unwrap()
            .iter::<model::Container>()
            .map(Result::unwrap)
            .filter(model::Container::selected)
            .for_each(|container| {
                if !visible_containers.contains(&container) {
                    container.set_selected(false);
                }
            });
    }
}
