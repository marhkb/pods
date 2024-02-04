use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::ops::Deref;

use adw::prelude::*;
use adw::subclass::prelude::*;
use futures::StreamExt;
use futures::stream;
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

const ACTION_CREATE_NETWORK: &str = "networks-panel.create-network";
const ACTION_PRUNE_UNUSED_NETWORKS: &str = "networks-panel.prune-unused-networks";
const ACTION_TOGGLE_NETWORKS_VIEW: &str = "networks-panel.toggle-networks-view";
const ACTION_ENTER_SELECTION_MODE: &str = "networks-panel.enter-selection-mode";
const ACTION_EXIT_SELECTION_MODE: &str = "networks-panel.exit-selection-mode";
const ACTION_SELECT_VISIBLE: &str = "networks-panel.select-visible";
const ACTION_SELECT_NONE: &str = "networks-panel.select-none";
const ACTION_DELETE_SELECTION: &str = "networks-panel.delete-selection";
const ACTION_TOGGLE_SORT_DIRECTION: &str = "networks-panel.toggle-sort-direction";
const ACTION_CHANGE_SORT_ATTRIBUTE: &str = "networks-panel.change-sort-attribute";
const ACTION_SHOW_ALL_NETWORKS: &str = "networks-panel.show-all-networks";

#[derive(Debug)]
pub(crate) struct Settings(gio::Settings);
impl Default for Settings {
    fn default() -> Self {
        Self(gio::Settings::new(&format!(
            "{}.view.panels.networks",
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
enum NetworksView {
    Grid(view::NetworksGridView),
    List(view::NetworksListView),
}

impl NetworksView {
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
#[enum_type(name = "NetworksPanelSortDirection")]
pub(crate) enum SortDirection {
    #[default]
    #[enum_value(nick = "asc")]
    Asc,
    #[enum_value(nick = "desc")]
    Desc,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "NetworksPanelSortAttribute")]
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
    #[properties(wrapper_type = super::NetworksPanel)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/networks_panel.ui")]
    pub(crate) struct NetworksPanel {
        pub(super) settings: Settings,
        pub(super) networks_view: RefCell<Option<NetworksView>>,
        pub(super) filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        pub(super) search_term: RefCell<String>,
        pub(super) model: RefCell<Option<gio::ListModel>>,
        #[property(get, set = Self::set_network_list, nullable)]
        pub(super) network_list: glib::WeakRef<model::NetworkList>,
        #[property(get, set)]
        pub(super) collapsed: Cell<bool>,
        #[property(get, set, builder(SortDirection::default()))]
        pub(super) sort_direction: RefCell<SortDirection>,
        #[property(get, set, builder(SortAttribute::default()))]
        pub(super) sort_attribute: RefCell<SortAttribute>,
        #[template_child]
        pub(super) create_network_button: TemplateChild<gtk::Button>,
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
        pub(super) create_network_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) prune_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) view_options_split_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) view_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) selected_networks_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) filter_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) networks_view_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) overhang_action_bar: TemplateChild<gtk::ActionBar>,
        #[template_child]
        pub(super) create_network_button_bottom_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) prune_button_bottom_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) view_options_split_button_bottom_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) view_button_bottom_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NetworksPanel {
        const NAME: &'static str = "PdsNetworksPanel";
        type Type = super::NetworksPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_CREATE_NETWORK,
            );
            klass.install_action(ACTION_CREATE_NETWORK, None, move |widget, _, _| {
                widget.create_network();
            });

            klass.install_action(ACTION_PRUNE_UNUSED_NETWORKS, None, |widget, _, _| {
                widget.show_prune_page();
            });

            klass.install_action(ACTION_TOGGLE_NETWORKS_VIEW, None, |widget, _, _| {
                widget.toggle_networks_view();
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

            klass.install_action_async(ACTION_DELETE_SELECTION, None, async |widget, _, _| {
                widget.delete_selection().await;
            });

            klass.install_action(ACTION_TOGGLE_SORT_DIRECTION, None, |widget, _, _| {
                widget.toggle_sort_direction();
            });
            klass.install_property_action(ACTION_CHANGE_SORT_ATTRIBUTE, "sort-attribute");

            klass.install_action(ACTION_SHOW_ALL_NETWORKS, None, |widget, _, _| {
                widget.set_search_mode(false);
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NetworksPanel {
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

            self.set_networks_view();
            self.settings.connect_changed(
                Some("view"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.imp().set_networks_view();
                    }
                ),
            );

            self.settings
                .bind("sort-direction", obj, "sort-direction")
                .build();
            self.settings
                .bind("sort-attribute", obj, "sort-attribute")
                .build();

            let network_list_expr = Self::Type::this_expression("network-list");
            let network_list_len_expr =
                network_list_expr.chain_property::<model::NetworkList>("len");
            let selection_mode_expr =
                network_list_expr.chain_property::<model::NetworkList>("selection-mode");
            let not_selection_mode_expr = selection_mode_expr.chain_closure::<bool>(closure!(
                |_: Self::Type, selection_mode: bool| { !selection_mode }
            ));
            let collapsed_expr = Self::Type::this_expression("collapsed");

            gtk::ClosureExpression::new::<Option<String>>(
                [
                    &network_list_len_expr,
                    &network_list_expr.chain_property::<model::NetworkList>("listing"),
                    &network_list_expr.chain_property::<model::NetworkList>("initialized"),
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
                            Some("networks")
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

            // TODO
            gtk::ClosureExpression::new::<String>(
                [
                    &network_list_len_expr,
                    &network_list_expr.chain_property::<model::AbstractContainerList>("running"),
                ],
                closure!(|_: Self::Type, len: u32, running: u32| {
                    if len == 0 {
                        String::new()
                    } else if len == 1 {
                        if running == 1 {
                            gettext("1 container, running")
                        } else {
                            gettext("1 container, stopped")
                        }
                    } else {
                        ngettext!(
                            "{} container total, {} running",
                            "{} containers total, {} running",
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

            network_list_expr
                .chain_property::<model::NetworkList>("num-selected")
                .chain_closure::<String>(closure!(|_: Self::Type, selected: u32| ngettext!(
                    "{} Selected Network",
                    "{} Selected Networks",
                    selected,
                    selected
                )))
                .bind(&self.selected_networks_button.get(), "label", Some(obj));

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

                    if term.is_empty() {
                        true
                    } else {
                        let network = item.downcast_ref::<model::Network>().unwrap().inner();
                        network.name.as_ref().unwrap().to_lowercase().contains(term)
                            || network.id.as_ref().unwrap().contains(term)
                    }
                }
            ));

            let sorter = gtk::CustomSorter::new(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                gtk::Ordering::Equal,
                move |item1, item2| {
                    let network1 = item1.downcast_ref::<model::Network>().unwrap();
                    let network2 = item2.downcast_ref::<model::Network>().unwrap();

                    let ordering = match obj.sort_attribute() {
                        SortAttribute::Name => network1
                            .inner()
                            .name
                            .as_ref()
                            .unwrap()
                            .to_lowercase()
                            .cmp(&network2.inner().name.as_ref().unwrap().to_lowercase()),
                        SortAttribute::Containers => {
                            // TODO
                            std::cmp::Ordering::Equal
                        }
                    };

                    match obj.sort_direction() {
                        SortDirection::Asc => ordering,
                        SortDirection::Desc => ordering.reverse(),
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

    impl WidgetImpl for NetworksPanel {}

    #[gtk::template_callbacks]
    impl NetworksPanel {
        fn set_networks_view(&self) {
            let model = self.model.borrow();
            let view = if self.settings.string("view") == "grid" {
                self.view_button.set_icon_name("view-list-symbolic");

                self.view_button
                    .set_tooltip_text(Some(&gettext("List View")));

                NetworksView::Grid(view::NetworksGridView::from(model.as_ref()))
            } else {
                self.view_button.set_icon_name("view-grid-symbolic");

                self.view_button
                    .set_tooltip_text(Some(&gettext("Grid View")));

                NetworksView::List(view::NetworksListView::from(model.as_ref()))
            };

            self.networks_view_bin.set_child(Some(view.view()));
            self.networks_view.replace(Some(view));
        }

        #[template_callback]
        fn on_notify_collapsed(&self) {
            if self.obj().collapsed() {
                self.create_network_button_top_bin
                    .set_child(gtk::Widget::NONE);
                self.prune_button_top_bin.set_child(gtk::Widget::NONE);
                self.view_options_split_button_top_bin
                    .set_child(gtk::Widget::NONE);
                self.view_button_top_bin.set_child(gtk::Widget::NONE);

                self.create_network_button_bottom_bin
                    .set_child(Some(&self.create_network_button.get()));
                self.prune_button_bottom_bin
                    .set_child(Some(&self.prune_button.get()));
                self.view_options_split_button_bottom_bin
                    .set_child(Some(&self.view_options_split_button.get()));
                self.view_button_bottom_bin
                    .set_child(Some(&self.view_button.get()));
            } else {
                self.create_network_button_bottom_bin
                    .set_child(gtk::Widget::NONE);
                self.prune_button_bottom_bin.set_child(gtk::Widget::NONE);
                self.view_options_split_button_bottom_bin
                    .set_child(gtk::Widget::NONE);
                self.view_button_bottom_bin.set_child(gtk::Widget::NONE);

                self.create_network_button_top_bin
                    .set_child(Some(&self.create_network_button.get()));
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

        pub(super) fn set_network_list(&self, value: &model::NetworkList) {
            let obj = &*self.obj();
            if obj.network_list().as_ref() == Some(value) {
                return;
            }

            obj.action_set_enabled(ACTION_DELETE_SELECTION, false);

            value.connect_notify_local(
                Some("num-selected"),
                clone!(
                    #[weak]
                    obj,
                    move |list, _| {
                        obj.action_set_enabled(ACTION_DELETE_SELECTION, list.num_selected() > 0);
                    }
                ),
            );

            let model = gtk::SortListModel::new(
                Some(gtk::FilterListModel::new(
                    Some(value.to_owned()),
                    self.filter.get().cloned(),
                )),
                self.sorter.get().cloned(),
            );

            if let Some(view) = &*self.networks_view.borrow() {
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
                        obj.deselect_hidden_networks(model.upcast_ref());
                    }
                }
            ));
            value.connect_initialized_notify(clone!(
                #[weak]
                obj,
                #[weak]
                model,
                move |network_list| obj
                    .imp()
                    .set_filter_stack_visible_child(network_list, &model)
            ));

            self.model.replace(Some(model.upcast()));

            self.network_list.set(Some(value));
        }

        fn set_filter_stack_visible_child(
            &self,
            network_list: &model::NetworkList,
            model: &impl IsA<gio::ListModel>,
        ) {
            self.filter_stack.set_visible_child_name(
                if model.n_items() > 0 || !network_list.initialized() {
                    "networks"
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
    pub(crate) struct NetworksPanel(ObjectSubclass<imp::NetworksPanel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for NetworksPanel {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl NetworksPanel {
    fn client(&self) -> Option<model::Client> {
        self.network_list()
            .as_ref()
            .and_then(model::NetworkList::client)
    }

    pub(crate) fn set_search_mode(&self, value: bool) {
        self.imp().search_bar.set_search_mode(value);
    }

    pub(crate) fn toggle_search_mode(&self) {
        self.set_search_mode(!self.imp().search_bar.is_search_mode());
    }

    pub(crate) fn create_network(&self) {
        if let Some(client) = self
            .network_list()
            .as_ref()
            .and_then(model::NetworkList::client)
        {
            utils::Dialog::new(self, &view::NetworkCreationPage::from(&client)).present();
        }
    }

    pub(crate) fn show_prune_page(&self) {
        if let Some(client) = self.client() {
            utils::Dialog::new(self, &view::ContainersPrunePage::from(&client)).present();
        }
    }

    pub(crate) fn toggle_networks_view(&self) {
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

    pub(crate) fn toggle_sort_direction(&self) {
        self.set_sort_direction(match self.sort_direction() {
            SortDirection::Asc => SortDirection::Desc,
            SortDirection::Desc => SortDirection::Asc,
        });
        self.imp().update_sorter();
    }

    pub(crate) fn enter_selection_mode(&self) {
        if let Some(list) = self.network_list().filter(|list| list.len() > 0) {
            list.select_none();
            list.set_selection_mode(true);
        }
    }

    pub(crate) fn exit_selection_mode(&self) {
        if let Some(list) = self.network_list() {
            list.set_selection_mode(false);
        }
    }

    pub(crate) fn select_visible(&self) {
        if let Some(view) = &*self.imp().networks_view.borrow() {
            view.select_visible();
        }
    }

    pub(crate) fn select_none(&self) {
        if let Some(list) = self.network_list().filter(|list| list.is_selection_mode()) {
            list.select_none();
        }
    }

    pub(crate) async fn delete_selection(&self) {
        let network_list = if let Some(network_list) = self.network_list() {
            network_list
        } else {
            return;
        };

        if network_list.num_selected() == 0 {
            return;
        }

        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Confirm Forced Deletion of Multiple Networks"))
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

        if "delete" != dialog.choose_future(self).await {
            return;
        }

        stream::iter(
            network_list
                .selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Network>().unwrap()),
        )
        .for_each(async |network| {
            if let Err(e) = network.delete(true).await {
                utils::show_error_toast(
                    self,
                    &gettext!(
                        "Error on deleting network '{}'",
                        network.inner().name.as_ref().unwrap()
                    ),
                    &e.to_string(),
                );
            }
        })
        .await;

        network_list.set_selection_mode(false);
        self.emit_by_name::<()>("exit-selection-mode", &[]);
    }

    fn deselect_hidden_networks(&self, model: &gio::ListModel) {
        let visible_networks = model
            .iter::<glib::Object>()
            .map(Result::unwrap)
            .map(|item| item.downcast::<model::Network>().unwrap())
            .collect::<Vec<_>>();

        self.network_list()
            .unwrap()
            .iter::<model::Network>()
            .map(Result::unwrap)
            .filter(model::Network::selected)
            .filter(|network| !visible_networks.contains(&network))
            .for_each(|network| network.set_selected(false));
    }
}
