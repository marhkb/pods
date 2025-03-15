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

const ACTION_CREATE_VOLUME: &str = "volumes-panel.create-volume";
const ACTION_PRUNE_VOLUMES: &str = "volumes-panel.prune-unused-volumes";
const ACTION_ENTER_SELECTION_MODE: &str = "volumes-panel.enter-selection-mode";
const ACTION_EXIT_SELECTION_MODE: &str = "volumes-panel.exit-selection-mode";
const ACTION_SELECT_VISIBLE: &str = "volumes-panel.select-visible";
const ACTION_SELECT_NONE: &str = "volumes-panel.select-none";
const ACTION_DELETE_SELECTION: &str = "volumes-panel.delete-selection";
const ACTION_TOGGLE_SORT_DIRECTION: &str = "volumes-panel.toggle-sort-direction";
const ACTION_CHANGE_SORT_ATTRIBUTE: &str = "volumes-panel.change-sort-attribute";
const ACTION_SHOW_ALL_VOLUMES: &str = "volumes-panel.show-all-volumes";

#[derive(Debug)]
pub(crate) struct Settings(gio::Settings);
impl Default for Settings {
    fn default() -> Self {
        Self(gio::Settings::new(&format!(
            "{}.view.panels.volumes",
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
#[enum_type(name = "VolumesPanelSortDirection")]
pub(crate) enum SortDirection {
    #[default]
    #[enum_value(nick = "asc")]
    Asc,
    #[enum_value(nick = "desc")]
    Desc,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "VolumesPanelSortAttribute")]
pub(crate) enum SortAttribute {
    #[default]
    #[enum_value(nick = "name")]
    Name,
    #[enum_value(nick = "age")]
    Age,
    #[enum_value(nick = "containers")]
    Containers,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::VolumesPanel)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/volumes_panel.ui")]
    pub(crate) struct VolumesPanel {
        pub(super) settings: Settings,
        pub(super) filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        pub(super) search_term: RefCell<String>,
        #[property(get, set = Self::set_volume_list)]
        pub(super) volume_list: glib::WeakRef<model::VolumeList>,
        #[property(get, set)]
        pub(super) collapsed: Cell<bool>,
        #[property(get, set, builder(SortDirection::default()))]
        pub(super) sort_direction: RefCell<SortDirection>,
        #[property(get, set, builder(SortAttribute::default()))]
        pub(super) sort_attribute: RefCell<SortAttribute>,
        #[template_child]
        pub(super) create_volume_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) prune_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) view_options_split_button: TemplateChild<adw::SplitButton>,
        #[property(get, set)]
        pub(super) show_only_used_volumes: Cell<bool>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) toolbar_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub(super) header_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) create_volume_menu_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) prune_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) view_options_split_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) selected_volumes_button: TemplateChild<gtk::MenuButton>,
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
        pub(super) create_volume_menu_button_bottom_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) prune_button_bottom_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) view_options_split_button_bottom_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumesPanel {
        const NAME: &'static str = "PdsVolumesPanel";
        type Type = super::VolumesPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_CREATE_VOLUME,
            );
            klass.install_action(ACTION_CREATE_VOLUME, None, move |widget, _, _| {
                widget.create_volume();
            });

            klass.install_action(ACTION_PRUNE_VOLUMES, None, |widget, _, _| {
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

            klass.install_action_async(ACTION_DELETE_SELECTION, None, async |widget, _, _| {
                widget.delete_selection().await;
            });

            klass.install_action(ACTION_TOGGLE_SORT_DIRECTION, None, |widget, _, _| {
                widget.toggle_sort_direction();
            });
            klass.install_property_action(ACTION_CHANGE_SORT_ATTRIBUTE, "sort-attribute");

            klass.install_action(ACTION_SHOW_ALL_VOLUMES, None, |widget, _, _| {
                widget.show_all_volumes();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VolumesPanel {
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

            let volume_list_expr = Self::Type::this_expression("volume-list");
            let volume_list_len_expr = volume_list_expr.chain_property::<model::VolumeList>("len");
            let selection_mode_expr =
                volume_list_expr.chain_property::<model::VolumeList>("selection-mode");
            let not_selection_mode_expr = selection_mode_expr.chain_closure::<bool>(closure!(
                |_: Self::Type, selection_mode: bool| { !selection_mode }
            ));
            let collapsed_expr = Self::Type::this_expression("collapsed");

            gtk::ClosureExpression::new::<Option<String>>(
                [
                    &volume_list_len_expr,
                    &volume_list_expr.chain_property::<model::VolumeList>("listing"),
                    &volume_list_expr.chain_property::<model::VolumeList>("initialized"),
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
                            Some("volumes")
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
                &[
                    volume_list_len_expr,
                    volume_list_expr.chain_property::<model::VolumeList>("unused"),
                ],
                closure!(|_: Self::Type, len: u32, unused: u32| {
                    if len == 0 {
                        String::new()
                    } else if len == 1 {
                        if unused == 1 {
                            gettext("1 volume, used")
                        } else {
                            gettext("1 volume, unused")
                        }
                    } else {
                        ngettext!(
                            "{} volumes total, {} unused",
                            "{} volumes total, {} unused",
                            len,
                            len,
                            unused,
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

            volume_list_expr
                .chain_property::<model::VolumeList>("num-selected")
                .chain_closure::<String>(closure!(|_: Self::Type, selected: u32| ngettext!(
                    "{} Selected Volume",
                    "{} Selected Volumes",
                    selected,
                    selected
                )))
                .bind(&self.selected_volumes_button.get(), "label", Some(obj));

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
                    item.downcast_ref::<model::Volume>()
                        .unwrap()
                        .inner()
                        .name
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
                    let volume1 = item1.downcast_ref::<model::Volume>().unwrap();
                    let volume2 = item2.downcast_ref::<model::Volume>().unwrap();

                    let ordering = match obj.sort_attribute() {
                        SortAttribute::Name => volume1
                            .inner()
                            .name
                            .to_lowercase()
                            .cmp(&volume2.inner().name.to_lowercase()),
                        SortAttribute::Age => {
                            let date1 = glib::DateTime::from_iso8601(
                                volume1.inner().created_at.as_deref().unwrap(),
                                None,
                            )
                            .unwrap();
                            let date2 = glib::DateTime::from_iso8601(
                                volume2.inner().created_at.as_deref().unwrap(),
                                None,
                            )
                            .unwrap();

                            date2.cmp(&date1)
                        }
                        SortAttribute::Containers => volume1
                            .container_list()
                            .len()
                            .cmp(&volume2.container_list().len()),
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

    impl WidgetImpl for VolumesPanel {}

    #[gtk::template_callbacks]
    impl VolumesPanel {
        #[template_callback]
        fn on_notify_collapsed(&self) {
            if self.obj().collapsed() {
                self.create_volume_menu_button_top_bin
                    .set_child(gtk::Widget::NONE);
                self.prune_button_top_bin.set_child(gtk::Widget::NONE);
                self.view_options_split_button_top_bin
                    .set_child(gtk::Widget::NONE);

                self.create_volume_menu_button_bottom_bin
                    .set_child(Some(&self.create_volume_button.get()));
                self.prune_button_bottom_bin
                    .set_child(Some(&self.prune_button.get()));
                self.view_options_split_button_bottom_bin
                    .set_child(Some(&self.view_options_split_button.get()));
            } else {
                self.create_volume_menu_button_bottom_bin
                    .set_child(gtk::Widget::NONE);
                self.prune_button_bottom_bin.set_child(gtk::Widget::NONE);
                self.view_options_split_button_bottom_bin
                    .set_child(gtk::Widget::NONE);

                self.create_volume_menu_button_top_bin
                    .set_child(Some(&self.create_volume_button.get()));
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
        fn on_notify_show_only_used_volumes(&self) {
            self.update_filter(if self.obj().show_only_used_volumes() {
                gtk::FilterChange::MoreStrict
            } else {
                gtk::FilterChange::LessStrict
            });
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

        pub(super) fn set_volume_list(&self, value: &model::VolumeList) {
            let obj = &*self.obj();
            if obj.volume_list().as_ref() == Some(value) {
                return;
            }

            value.connect_containers_of_volume_changed(clone!(
                #[weak]
                obj,
                move |_, _| if obj.sort_attribute() == SortAttribute::Containers {
                    obj.imp().update_sorter();
                }
            ));

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

            value.connect_notify_local(
                Some("used"),
                clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.imp().update_filter(gtk::FilterChange::Different);
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

            self.list_box.bind_model(Some(&model), |item| {
                view::VolumeRow::from(item.downcast_ref().unwrap()).upcast()
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
                        obj.deselect_hidden_volumes(model.upcast_ref());
                    }
                }
            ));
            value.connect_initialized_notify(clone!(
                #[weak]
                obj,
                #[weak]
                model,
                move |volume_list| obj
                    .imp()
                    .set_filter_stack_visible_child(volume_list, &model)
            ));

            self.volume_list.set(Some(value));
        }

        fn set_filter_stack_visible_child(
            &self,
            volume_list: &model::VolumeList,
            model: &impl IsA<gio::ListModel>,
        ) {
            self.filter_stack.set_visible_child_name(
                if model.n_items() > 0 || !volume_list.initialized() {
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
    pub(crate) struct VolumesPanel(ObjectSubclass<imp::VolumesPanel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for VolumesPanel {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl VolumesPanel {
    pub(crate) fn toggle_sort_direction(&self) {
        self.set_sort_direction(match self.sort_direction() {
            SortDirection::Asc => SortDirection::Desc,
            SortDirection::Desc => SortDirection::Asc,
        });
        self.imp().update_sorter();
    }

    pub(crate) fn show_all_volumes(&self) {
        self.set_show_only_used_volumes(false);
        self.set_search_mode(false);
    }

    pub(crate) fn set_search_mode(&self, value: bool) {
        self.imp().search_bar.set_search_mode(value);
    }

    pub(crate) fn toggle_search_mode(&self) {
        self.set_search_mode(!self.imp().search_bar.is_search_mode());
    }

    pub(crate) fn create_volume(&self) {
        if let Some(client) = self
            .volume_list()
            .as_ref()
            .and_then(model::VolumeList::client)
        {
            utils::Dialog::new(self, &view::VolumeCreationPage::from(&client)).present();
        }
    }

    pub(crate) fn show_prune_page(&self) {
        if let Some(client) = self.volume_list().and_then(|list| list.client()) {
            utils::Dialog::new(self, &view::VolumesPrunePage::from(&client))
                .follows_content_size(true)
                .present();
        }
    }

    pub(crate) fn enter_selection_mode(&self) {
        if let Some(list) = self.volume_list().filter(|list| list.len() > 0) {
            list.select_none();
            list.set_selection_mode(true);
        }
    }

    pub(crate) fn exit_selection_mode(&self) {
        if let Some(list) = self.volume_list() {
            list.set_selection_mode(false);
        }
    }

    pub(crate) fn select_visible(&self) {
        (0..)
            .map(|pos| self.imp().list_box.row_at_index(pos))
            .take_while(Option::is_some)
            .flatten()
            .for_each(|row| {
                row.downcast_ref::<view::VolumeRow>()
                    .unwrap()
                    .volume()
                    .unwrap()
                    .set_selected(row.is_visible());
            });
    }

    pub(crate) fn select_none(&self) {
        if let Some(list) = self.volume_list().filter(|list| list.is_selection_mode()) {
            list.select_none();
        }
    }

    pub(crate) async fn delete_selection(&self) {
        let volume_list = if let Some(volume_list) = self.volume_list() {
            volume_list
        } else {
            return;
        };

        if volume_list.num_selected() == 0 {
            return;
        }

        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Confirm Forced Deletion of Multiple Volumes"))
            .body(gettext(
                "There may be containers associated with some of the volumes, which will also be removed!",
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
            volume_list
                .selected_items()
                .iter()
                .map(|obj| obj.downcast_ref::<model::Volume>().unwrap()),
        )
        .for_each(async |volume| {
            if let Err(e) = volume.delete(true).await {
                utils::show_error_toast(
                    self,
                    &gettext!("Error on deleting volume '{}'", volume.inner().name),
                    &e.to_string(),
                );
            }
        })
        .await;

        volume_list.set_selection_mode(false);
        self.emit_by_name::<()>("exit-selection-mode", &[]);
    }

    fn deselect_hidden_volumes(&self, model: &gio::ListModel) {
        let visible_volumes = model
            .iter::<glib::Object>()
            .map(Result::unwrap)
            .map(|item| item.downcast::<model::Volume>().unwrap())
            .collect::<Vec<_>>();

        self.volume_list()
            .unwrap()
            .iter::<model::Volume>()
            .map(Result::unwrap)
            .filter(model::Volume::selected)
            .for_each(|volume| {
                if !visible_volumes.contains(&volume) {
                    volume.set_selected(false);
                }
            });
    }
}
