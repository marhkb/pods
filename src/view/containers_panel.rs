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
use crate::model::AbstractContainerListExt;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

const ACTION_CREATE_CONTAINER: &str = "containers-panel.create-container";
const ACTION_PRUNE_UNUSED_CONTAINERS: &str = "containers-panel.prune-unused-containers";
const ACTION_TOGGLE_CONTAINERS_VIEW: &str = "containers-panel.toggle-containers-view";
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
const ACTION_TOGGLE_SORT_DIRECTION: &str = "containers-panel.toggle-sort-direction";
const ACTION_CHANGE_SORT_ATTRIBUTE: &str = "containers-panel.change-sort-attribute";
const ACTION_TOGGLE_SHOW_RUNNING_CONTAINERS_FIRST: &str =
    "containers-panel.toggle-show-running-containers-first";
const ACTION_SHOW_ALL_CONTAINERS: &str = "containers-panel.show-all-containers";

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
            "{}.view.panels.containers",
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

#[derive(Debug)]
enum ContainersView {
    Grid(view::ContainersGridView),
    List(view::ContainersListView),
}

impl ContainersView {
    fn view(&self) -> &gtk::Widget {
        match self {
            Self::Grid(view) => view.upcast_ref(),
            Self::List(view) => view.upcast_ref(),
        }
    }

    fn set_model(&self, model: Option<&gio::ListModel>) {
        match self {
            Self::Grid(view) => view.set_model(model),
            Self::List(view) => view.set_model(model),
        }
    }

    pub(crate) fn select_visible(&self) {
        match self {
            Self::Grid(view) => view.select_visible(),
            Self::List(view) => view.select_visible(),
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ContainersPanelSortDirection")]
pub(crate) enum SortDirection {
    #[default]
    #[enum_value(nick = "asc")]
    Asc,
    #[enum_value(nick = "desc")]
    Desc,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ContainersPanelSortAttribute")]
pub(crate) enum SortAttribute {
    #[default]
    #[enum_value(nick = "name")]
    Name,
    #[enum_value(nick = "image")]
    Image,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainersPanel)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/containers_panel.ui")]
    pub(crate) struct ContainersPanel {
        pub(super) settings: Settings,
        pub(super) containers_view: RefCell<Option<ContainersView>>,
        pub(super) filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        pub(super) search_term: RefCell<String>,
        pub(super) model: RefCell<Option<gio::ListModel>>,
        #[property(get, set = Self::set_container_list, nullable)]
        pub(super) container_list: glib::WeakRef<model::ContainerList>,
        #[property(get, set)]
        pub(super) collapsed: Cell<bool>,
        #[property(get, set, builder(SortDirection::default()))]
        pub(super) sort_direction: RefCell<SortDirection>,
        #[property(get, set, builder(SortAttribute::default()))]
        pub(super) sort_attribute: RefCell<SortAttribute>,
        #[property(get, set)]
        pub(super) show_running_containers_first: Cell<bool>,
        #[template_child]
        pub(super) create_container_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) prune_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) view_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) view_options_split_button: TemplateChild<adw::SplitButton>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) toolbar_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub(super) header_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) create_container_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) prune_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) view_options_split_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) view_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) selected_containers_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) filter_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) containers_view_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) overhang_action_bar: TemplateChild<gtk::ActionBar>,
        #[template_child]
        pub(super) create_container_button_bottom_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) prune_button_bottom_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) view_options_split_button_bottom_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) view_button_bottom_bin: TemplateChild<adw::Bin>,
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

            klass.install_action(ACTION_TOGGLE_CONTAINERS_VIEW, None, |widget, _, _| {
                widget.toggle_containers_view();
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

            klass.install_action(ACTION_TOGGLE_SORT_DIRECTION, None, |widget, _, _| {
                widget.toggle_sort_direction();
            });
            klass.install_property_action(ACTION_CHANGE_SORT_ATTRIBUTE, "sort-attribute");
            klass.install_property_action(
                ACTION_TOGGLE_SHOW_RUNNING_CONTAINERS_FIRST,
                "show-running-containers-first",
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

            self.set_containers_view();
            self.settings.connect_changed(
                Some("view"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.imp().set_containers_view();
                    }
                ),
            );

            self.settings
                .bind("sort-direction", obj, "sort-direction")
                .build();
            self.settings
                .bind("sort-attribute", obj, "sort-attribute")
                .build();

            self.settings
                .bind("show-running-first", obj, "show-running-containers-first")
                .build();

            let container_list_expr = Self::Type::this_expression("container-list");
            let container_list_containers_expr =
                container_list_expr.chain_property::<model::ContainerList>("containers");
            let selection_mode_expr =
                container_list_expr.chain_property::<model::ContainerList>("selection-mode");
            let not_selection_mode_expr = selection_mode_expr.chain_closure::<bool>(closure!(
                |_: Self::Type, selection_mode: bool| { !selection_mode }
            ));
            let collapsed_expr = Self::Type::this_expression("collapsed");

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
                    if !selection_mode { "main" } else { "selection" }
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

            let filter = gtk::EveryFilter::new();
            filter.append(
                gtk::BoolFilter::builder()
                    .expression(model::Container::this_expression("is-infra"))
                    .invert(true)
                    .build(),
            );
            filter.append(gtk::CustomFilter::new(clone!(
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
            )));

            let sorter = gtk::CustomSorter::new(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                gtk::Ordering::Equal,
                move |item1, item2| {
                    let container1 = item1.downcast_ref::<model::Container>().unwrap();
                    let container2 = item2.downcast_ref::<model::Container>().unwrap();

                    if obj.show_running_containers_first() {
                        match container2.status().cmp(&container1.status()) {
                            std::cmp::Ordering::Equal => obj.imp().ordering(container1, container2),
                            other => other,
                        }
                    } else {
                        obj.imp().ordering(container1, container2)
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

    impl WidgetImpl for ContainersPanel {}

    #[gtk::template_callbacks]
    impl ContainersPanel {
        fn ordering(
            &self,
            container1: &model::Container,
            container2: &model::Container,
        ) -> std::cmp::Ordering {
            let obj = self.obj();
            let ordering = match obj.sort_attribute() {
                SortAttribute::Name => container1
                    .name()
                    .to_lowercase()
                    .cmp(&container2.name().to_lowercase()),
                SortAttribute::Image => container1.image_name().cmp(&container2.image_name()),
            };

            match obj.sort_direction() {
                SortDirection::Asc => ordering,
                SortDirection::Desc => ordering.reverse(),
            }
        }

        fn set_containers_view(&self) {
            let model = self.model.borrow();
            let view = if self.settings.string("view") == "grid" {
                self.view_button.set_icon_name("view-list-symbolic");

                self.view_button
                    .set_tooltip_text(Some(&gettext("List View")));

                ContainersView::Grid(view::ContainersGridView::from(model.as_ref()))
            } else {
                self.view_button.set_icon_name("view-grid-symbolic");

                self.view_button
                    .set_tooltip_text(Some(&gettext("Grid View")));

                ContainersView::List(view::ContainersListView::from(model.as_ref()))
            };

            self.containers_view_bin.set_child(Some(view.view()));
            self.containers_view.replace(Some(view));
        }

        #[template_callback]
        fn on_notify_collapsed(&self) {
            if self.obj().collapsed() {
                self.create_container_button_top_bin
                    .set_child(gtk::Widget::NONE);
                self.prune_button_top_bin.set_child(gtk::Widget::NONE);
                self.view_options_split_button_top_bin
                    .set_child(gtk::Widget::NONE);
                self.view_button_top_bin.set_child(gtk::Widget::NONE);

                self.create_container_button_bottom_bin
                    .set_child(Some(&self.create_container_button.get()));
                self.prune_button_bottom_bin
                    .set_child(Some(&self.prune_button.get()));
                self.view_options_split_button_bottom_bin
                    .set_child(Some(&self.view_options_split_button.get()));
                self.view_button_bottom_bin
                    .set_child(Some(&self.view_button.get()));
            } else {
                self.create_container_button_bottom_bin
                    .set_child(gtk::Widget::NONE);
                self.prune_button_bottom_bin.set_child(gtk::Widget::NONE);
                self.view_options_split_button_bottom_bin
                    .set_child(gtk::Widget::NONE);
                self.view_button_bottom_bin.set_child(gtk::Widget::NONE);

                self.create_container_button_top_bin
                    .set_child(Some(&self.create_container_button.get()));
                self.prune_button_top_bin
                    .set_child(Some(&self.prune_button.get()));
                self.view_options_split_button_top_bin
                    .set_child(Some(&self.view_options_split_button.get()));
                self.view_button_top_bin
                    .set_child(Some(&self.view_button.get()));
            }
        }

        #[template_callback]
        fn on_notify_sort_attribute(&self) {
            self.update_sorter();
        }

        #[template_callback]
        fn on_notify_show_running_containers_first(&self) {
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

        pub(super) fn set_container_list(&self, value: &model::ContainerList) {
            let obj = &*self.obj();
            if obj.container_list().as_ref() == Some(value) {
                return;
            }

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

            value.connect_notify_local(
                Some("running"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| obj.imp().update_filter(gtk::FilterChange::Different)
                ),
            );

            value.connect_container_name_changed(clone!(
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
                    Some(value.to_owned()),
                    self.filter.get().cloned(),
                )),
                self.sorter.get().cloned(),
            );

            if let Some(view) = &*self.containers_view.borrow() {
                view.set_model(Some(model.upcast_ref()));
            }

            self.set_filter_stack_visible_child(value, &model);
            model.connect_items_changed(clone!(
                #[weak]
                obj,
                #[weak]
                value,
                move |model, _, removed, _| {
                    obj.imp().set_filter_stack_visible_child(&value, model);

                    if removed > 0 {
                        obj.deselect_hidden_containers(model.upcast_ref());
                    }
                }
            ));
            value.connect_initialized_notify(clone!(
                #[weak]
                obj,
                #[weak]
                model,
                move |container_list| obj
                    .imp()
                    .set_filter_stack_visible_child(container_list, &model)
            ));

            self.model.replace(Some(model.upcast()));

            self.container_list.set(Some(value));
        }

        fn set_filter_stack_visible_child(
            &self,
            container_list: &model::ContainerList,
            model: &impl IsA<gio::ListModel>,
        ) {
            self.filter_stack.set_visible_child_name(
                if model.n_items() > 0 || !container_list.initialized() {
                    "containers"
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

    pub(crate) fn toggle_sort_direction(&self) {
        self.set_sort_direction(match self.sort_direction() {
            SortDirection::Asc => SortDirection::Desc,
            SortDirection::Desc => SortDirection::Asc,
        });
        self.imp().update_sorter();
    }

    pub(crate) fn show_all_containers(&self) {
        self.set_show_running_containers_first(false);
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

    pub(crate) fn toggle_containers_view(&self) {
        let settings = &self.imp().settings;
        settings
            .set_string(
                "view",
                if settings.string("view") == "grid" {
                    "list"
                } else {
                    "grid"
                },
            )
            .unwrap();
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
        if let Some(view) = &*self.imp().containers_view.borrow() {
            view.select_visible();
        }
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
