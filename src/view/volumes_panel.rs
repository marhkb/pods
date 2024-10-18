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
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

const ACTION_CREATE_VOLUME: &str = "volumes-panel.create-volume";
const ACTION_PRUNE_VOLUMES: &str = "volumes-panel.prune-volumes";
const ACTION_ENTER_SELECTION_MODE: &str = "volumes-panel.enter-selection-mode";
const ACTION_EXIT_SELECTION_MODE: &str = "volumes-panel.exit-selection-mode";
const ACTION_SELECT_VISIBLE: &str = "volumes-panel.select-visible";
const ACTION_SELECT_NONE: &str = "volumes-panel.select-none";
const ACTION_DELETE_SELECTION: &str = "volumes-panel.delete-selection";
const ACTION_TOGGLE_SHOW_ONLY_USED_VOLUMES: &str = "volumes-panel.toggle-show-only-used-volumes";
const ACTION_SHOW_ALL_VOLUMES: &str = "volumes-panel.show-all-volumes";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::VolumesPanel)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/volumes_panel.ui")]
    pub(crate) struct VolumesPanel {
        pub(super) settings: utils::PodsSettings,
        pub(super) filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        pub(super) search_term: RefCell<String>,
        #[property(get, set = Self::set_volume_list)]
        pub(super) volume_list: glib::WeakRef<model::VolumeList>,
        #[property(get, set)]
        pub(super) collapsed: Cell<bool>,
        #[property(get, set)]
        pub(super) show_only_used_volumes: Cell<bool>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) toolbar_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub(super) header_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
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

            klass.install_action(ACTION_DELETE_SELECTION, None, |widget, _, _| {
                widget.delete_selection();
            });

            klass.install_property_action(
                ACTION_TOGGLE_SHOW_ONLY_USED_VOLUMES,
                "show-only-used-volumes",
            );

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
                .bind("show-only-used-volumes", obj, "show-only-used-volumes")
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
                    if !selection_mode {
                        "main"
                    } else {
                        "selection"
                    }
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

            let search_filter = gtk::CustomFilter::new(clone!(
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

            let state_filter = gtk::AnyFilter::new();
            state_filter.append(gtk::CustomFilter::new(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                false,
                move |_| !obj.show_only_used_volumes()
            )));
            state_filter.append(gtk::BoolFilter::new(Some(
                model::Volume::this_expression("container-list")
                    .chain_property::<model::SimpleContainerList>("len")
                    .chain_closure::<bool>(closure!(|_: model::Volume, len: u32| len > 0)),
            )));

            let filter = gtk::EveryFilter::new();
            filter.append(search_filter);
            filter.append(state_filter);

            let sorter = gtk::StringSorter::new(Some(
                model::Volume::this_expression("inner").chain_closure::<String>(closure!(
                    |_: model::Volume, inner: model::BoxedVolume| inner.name.clone()
                )),
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

    pub(crate) fn delete_selection(&self) {
        if self
            .volume_list()
            .map(|list| list.num_selected())
            .unwrap_or(0)
            == 0
        {
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

        dialog.connect_response(
            None,
            clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, response| if response == "delete" {
                    if let Some(list) = obj.volume_list() {
                        list.selected_items()
                            .iter()
                            .map(|obj| obj.downcast_ref::<model::Volume>().unwrap())
                            .for_each(|volume| {
                                volume.delete(
                                    true,
                                    clone!(
                                        #[weak]
                                        obj,
                                        move |volume, result| {
                                            if let Err(e) = result {
                                                utils::show_error_toast(
                                                    &obj,
                                                    &gettext!(
                                                        "Error on deleting volume '{}'",
                                                        volume.inner().name
                                                    ),
                                                    &e.to_string(),
                                                );
                                            }
                                        }
                                    ),
                                );
                            });
                        list.set_selection_mode(false);
                        obj.emit_by_name::<()>("exit-selection-mode", &[]);
                    }
                }
            ),
        );

        dialog.present(Some(self));
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
