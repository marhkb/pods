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

const ACTION_PULL_IMAGE: &str = "images-panel.pull-image";
const ACTION_BUILD_IMAGE: &str = "images-panel.build-image";
const ACTION_PRUNE_UNUSED_IMAGES: &str = "images-panel.prune-unused-images";
const ACTION_ENTER_SELECTION_MODE: &str = "images-panel.enter-selection-mode";
const ACTION_EXIT_SELECTION_MODE: &str = "images-panel.exit-selection-mode";
const ACTION_SELECT_VISIBLE: &str = "images-panel.select-visible";
const ACTION_SELECT_NONE: &str = "images-panel.select-none";
const ACTION_DELETE_SELECTION: &str = "images-panel.delete-selection";
const ACTION_TOGGLE_SORT_DIRECTION: &str = "images-panel.toggle-sort-direction";
const ACTION_CHANGE_SORT_ATTRIBUTE: &str = "images-panel.change-sort-attribute";
const ACTION_SHOW_ALL_IMAGES: &str = "images-panel.show-all-images";

#[derive(Debug)]
struct Settings(gio::Settings);
impl Default for Settings {
    fn default() -> Self {
        Self(gio::Settings::new(&format!(
            "{}.view.panels.images",
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
#[enum_type(name = "ImagesPanelSortDirection")]
pub(crate) enum SortDirection {
    #[default]
    #[enum_value(nick = "asc")]
    Asc,
    #[enum_value(nick = "desc")]
    Desc,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ImagesPanelSortAttribute")]
pub(crate) enum SortAttribute {
    #[default]
    #[enum_value(nick = "name")]
    Name,
    #[enum_value(nick = "tag")]
    Tag,
    #[enum_value(nick = "containers")]
    Containers,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImagesPanel)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/images_panel.ui")]
    pub(crate) struct ImagesPanel {
        pub(super) settings: Settings,
        pub(super) filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        pub(super) search_term: RefCell<String>,
        #[property(get, set = Self::set_image_list, nullable)]
        pub(super) image_list: glib::WeakRef<model::ImageList>,
        #[property(get, set)]
        pub(super) collapsed: Cell<bool>,
        #[property(get, set, builder(SortDirection::default()))]
        pub(super) sort_direction: RefCell<SortDirection>,
        #[property(get, set, builder(SortAttribute::default()))]
        pub(super) sort_attribute: RefCell<SortAttribute>,
        #[template_child]
        pub(super) create_image_menu_button: TemplateChild<gtk::MenuButton>,
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
        pub(super) create_image_menu_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) prune_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) view_options_split_button_top_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) selected_images_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) images_group: TemplateChild<adw::PreferencesGroup>,
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
        pub(super) create_image_menu_button_bottom_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) prune_button_bottom_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) view_options_split_button_bottom_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagesPanel {
        const NAME: &'static str = "PdsImagesPanel";
        type Type = super::ImagesPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_PULL_IMAGE,
            );
            klass.install_action(ACTION_PULL_IMAGE, None, |widget, _, _| {
                widget.show_download_page();
            });
            klass.install_action(ACTION_BUILD_IMAGE, None, |widget, _, _| {
                widget.show_build_page();
            });

            klass.install_action(ACTION_PRUNE_UNUSED_IMAGES, None, |widget, _, _| {
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

            klass.install_action(ACTION_TOGGLE_SORT_DIRECTION, None, |widget, _, _| {
                widget.toggle_sort_direction();
            });
            klass.install_property_action(ACTION_CHANGE_SORT_ATTRIBUTE, "sort-attribute");

            klass.install_action(ACTION_SHOW_ALL_IMAGES, None, |widget, _, _| {
                widget.show_all_images();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagesPanel {
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

            let image_list_expr = Self::Type::this_expression("image-list");
            let image_list_len_expr = image_list_expr.chain_property::<model::ImageList>("len");
            let selection_mode_expr =
                image_list_expr.chain_property::<model::ImageList>("selection-mode");
            let not_selection_mode_expr = selection_mode_expr.chain_closure::<bool>(closure!(
                |_: Self::Type, selection_mode: bool| { !selection_mode }
            ));
            let collapsed_expr = Self::Type::this_expression("collapsed");

            gtk::ClosureExpression::new::<Option<String>>(
                [
                    &image_list_len_expr,
                    &image_list_expr.chain_property::<model::ImageList>("listing"),
                    &image_list_expr.chain_property::<model::ImageList>("initialized"),
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
                            Some("images")
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
                    &image_list_len_expr,
                    &image_list_expr.chain_property::<model::ImageList>("intermediates"),
                ],
                closure!(|obj: Self::Type, len: u32, intermediates: u32| {
                    match obj.image_list() {
                        Some(list) => {
                            if len == 0 {
                                String::new()
                            } else if len == 1 {
                                if intermediates == 0 {
                                    gettext("1 image, used")
                                } else {
                                    gettext("1 image, unused")
                                }
                            } else {
                                ngettext!(
                                    "{} image total ({}), {} unused ({})",
                                    "{} images total ({}), {} unused ({})",
                                    len,
                                    len,
                                    glib::format_size(list.total_size()),
                                    intermediates,
                                    glib::format_size(list.unused_size()),
                                )
                            }
                        }
                        None => String::new(),
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

            image_list_expr
                .chain_property::<model::ImageList>("num-selected")
                .chain_closure::<String>(closure!(|_: Self::Type, selected: u32| ngettext!(
                    "{} Selected Image",
                    "{} Selected Images",
                    selected,
                    selected
                )))
                .bind(&self.selected_images_button.get(), "label", Some(obj));

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
                    let image = item.downcast_ref::<model::Image>().unwrap();
                    let term = &*obj.imp().search_term.borrow();

                    image.id().contains(term) || image.repo_tags().contains(term)
                }
            ));

            let sorter = gtk::CustomSorter::new(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                gtk::Ordering::Equal,
                move |item1, item2| {
                    let image1 = item1.downcast_ref::<model::Image>().unwrap();
                    let image2 = item2.downcast_ref::<model::Image>().unwrap();

                    let ordering = match obj.sort_attribute() {
                        SortAttribute::Name => {
                            image1.id().to_lowercase().cmp(&image2.id().to_lowercase())
                        }
                        SortAttribute::Tag => image1
                            .repo_tags()
                            .get(0)
                            .map(|repo_tag| repo_tag.full())
                            .cmp(&image2.repo_tags().get(0).map(|repo_tag| repo_tag.full())),
                        SortAttribute::Containers => image1.containers().cmp(&image2.containers()),
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

    impl WidgetImpl for ImagesPanel {}

    #[gtk::template_callbacks]
    impl ImagesPanel {
        #[template_callback]
        fn on_notify_collapsed(&self) {
            if self.obj().collapsed() {
                self.create_image_menu_button_top_bin
                    .set_child(gtk::Widget::NONE);
                self.prune_button_top_bin.set_child(gtk::Widget::NONE);
                self.view_options_split_button_top_bin
                    .set_child(gtk::Widget::NONE);

                self.create_image_menu_button_bottom_bin
                    .set_child(Some(&self.create_image_menu_button.get()));
                self.prune_button_bottom_bin
                    .set_child(Some(&self.prune_button.get()));
                self.view_options_split_button_bottom_bin
                    .set_child(Some(&self.view_options_split_button.get()));
            } else {
                self.create_image_menu_button_bottom_bin
                    .set_child(gtk::Widget::NONE);
                self.prune_button_bottom_bin.set_child(gtk::Widget::NONE);
                self.view_options_split_button_bottom_bin
                    .set_child(gtk::Widget::NONE);

                self.create_image_menu_button_top_bin
                    .set_child(Some(&self.create_image_menu_button.get()));
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

        pub(super) fn set_image_list(&self, value: &model::ImageList) {
            let obj = &*self.obj();
            if obj.image_list().as_ref() == Some(value) {
                return;
            }

            value.connect_containers_of_image_changed(clone!(
                #[weak]
                obj,
                move |_, _| {
                    glib::timeout_add_seconds_local_once(
                        1,
                        clone!(
                            #[weak]
                            obj,
                            move || if obj.sort_attribute() == SortAttribute::Name {
                                obj.imp().update_sorter();
                            }
                        ),
                    );
                }
            ));

            value.connect_tags_of_image_changed(clone!(
                #[weak]
                obj,
                move |_, _| {
                    glib::timeout_add_seconds_local_once(
                        1,
                        clone!(
                            #[weak]
                            obj,
                            move || if obj.sort_attribute() == SortAttribute::Tag {
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
                view::ImageRow::from(item.downcast_ref().unwrap()).upcast()
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
                        obj.deselect_hidden_images(model.upcast_ref());
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

            self.image_list.set(Some(value));
        }

        fn set_filter_stack_visible_child(
            &self,
            image_list: &model::ImageList,
            model: &impl IsA<gio::ListModel>,
        ) {
            self.filter_stack.set_visible_child_name(
                if model.n_items() > 0 || !image_list.initialized() {
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
    pub(crate) struct ImagesPanel(ObjectSubclass<imp::ImagesPanel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ImagesPanel {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl ImagesPanel {
    pub(crate) fn toggle_sort_direction(&self) {
        self.set_sort_direction(match self.sort_direction() {
            SortDirection::Asc => SortDirection::Desc,
            SortDirection::Desc => SortDirection::Asc,
        });
        self.imp().update_sorter();
    }

    pub(crate) fn show_all_images(&self) {
        self.set_search_mode(false);
    }

    pub(crate) fn set_search_mode(&self, value: bool) {
        self.imp().search_bar.set_search_mode(value);
    }

    pub(crate) fn toggle_search_mode(&self) {
        self.set_search_mode(!self.imp().search_bar.is_search_mode());
    }

    pub(crate) fn show_download_page(&self) {
        if let Some(client) = self.client() {
            utils::Dialog::new(self, &view::ImagePullPage::from(&client))
                .height(640)
                .present();
        }
    }

    pub(crate) fn show_build_page(&self) {
        if let Some(client) = self.client() {
            utils::Dialog::new(self, &view::ImageBuildPage::from(&client)).present();
        }
    }

    pub(crate) fn show_prune_page(&self) {
        if let Some(client) = self.client() {
            utils::Dialog::new(self, &view::ImagesPrunePage::from(&client))
                .follows_content_size(true)
                .present();
        }
    }

    pub(crate) fn enter_selection_mode(&self) {
        if let Some(list) = self.image_list().filter(|list| list.len() > 0) {
            list.select_none();
            list.set_selection_mode(true);
        }
    }

    pub(crate) fn exit_selection_mode(&self) {
        if let Some(list) = self.image_list() {
            list.set_selection_mode(false);
        }
    }

    pub(crate) fn select_visible(&self) {
        (0..)
            .map(|pos| self.imp().list_box.row_at_index(pos))
            .take_while(Option::is_some)
            .flatten()
            .for_each(|row| {
                row.downcast_ref::<view::ImageRow>()
                    .unwrap()
                    .image()
                    .unwrap()
                    .set_selected(row.is_visible());
            });
    }

    pub(crate) fn select_none(&self) {
        if let Some(list) = self.image_list().filter(|list| list.is_selection_mode()) {
            list.select_none();
        }
    }

    pub(crate) fn delete_selection(&self) {
        if self
            .image_list()
            .map(|list| list.num_selected())
            .unwrap_or(0)
            == 0
        {
            return;
        }

        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Confirm Forced Deletion of Multiple Images"))
            .body(gettext(
                "There may be containers associated with those images, which will also be removed!",
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
                move |_, response| if response == "delete"
                    && let Some(list) = obj.image_list()
                {
                    list.selected_items()
                        .iter()
                        .map(|obj| obj.downcast_ref::<model::Image>().unwrap())
                        .for_each(|image| {
                            image.delete(clone!(
                                #[weak]
                                obj,
                                move |image, result| {
                                    if let Err(e) = result {
                                        utils::show_error_toast(
                                            &obj,
                                            // Translators: The first "{}" is a placeholder for the image id, the second is for an error message.
                                            &gettext!("Error on deleting image '{}'", image.id()),
                                            &e.to_string(),
                                        );
                                    }
                                }
                            ));
                        });
                    list.set_selection_mode(false);
                }
            ),
        );

        dialog.present(Some(self));
    }

    fn deselect_hidden_images(&self, model: &gio::ListModel) {
        let visible_images = model
            .iter::<glib::Object>()
            .map(Result::unwrap)
            .map(|item| item.downcast::<model::Image>().unwrap())
            .collect::<Vec<_>>();

        self.image_list()
            .unwrap()
            .iter::<model::Image>()
            .map(Result::unwrap)
            .filter(model::Image::selected)
            .for_each(|image| {
                if !visible_images.contains(&image) {
                    image.set_selected(false);
                }
            });
    }

    fn client(&self) -> Option<model::Client> {
        self.image_list()
            .as_ref()
            .and_then(model::ImageList::client)
    }
}
