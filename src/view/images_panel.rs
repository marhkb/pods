use std::cell::RefCell;

use adw::prelude::MessageDialogExtManual;
use adw::traits::MessageDialogExt;
use gettextrs::gettext;
use gettextrs::ngettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::unsync::OnceCell as UnsyncOnceCell;

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

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImagesPanel)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/images_panel.ui")]
    pub(crate) struct ImagesPanel {
        pub(super) settings: utils::PodsSettings,
        pub(super) filter: UnsyncOnceCell<gtk::Filter>,
        pub(super) sorter: UnsyncOnceCell<gtk::Sorter>,
        pub(super) search_term: RefCell<String>,
        #[property(get, set = Self::set_image_list, nullable)]
        pub(super) image_list: glib::WeakRef<model::ImageList>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) main_header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) hide_intermediates_toggle_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) selection_header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub(super) selected_images_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) images_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
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
                None,
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
                .bind(
                    "hide-intermediate-images",
                    &self.hide_intermediates_toggle_button.get(),
                    "active",
                )
                .build();

            let image_list_expr = Self::Type::this_expression("image-list");
            let image_list_len_expr = image_list_expr.chain_property::<model::ImageList>("len");
            let selection_mode_expr =
                image_list_expr.chain_property::<model::ImageList>("selection-mode");
            let not_selection_mode_expr = selection_mode_expr.chain_closure::<bool>(closure!(
                |_: Self::Type, selection_mode: bool| { !selection_mode }
            ));

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

            not_selection_mode_expr.bind(&self.main_header_bar.get(), "visible", Some(obj));

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

            selection_mode_expr.bind(&self.selection_header_bar.get(), "visible", Some(obj));

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

            let search_filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    let image = item.downcast_ref::<model::Image>().unwrap();
                    let term = &*obj.imp().search_term.borrow();

                    image.id().contains(term) || image.repo_tags().contains(term)
                }));

            let state_filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    !obj.imp().hide_intermediates_toggle_button.is_active()
                    || item
                        .downcast_ref::<model::Image>()
                        .unwrap()
                        .repo_tags()
                        .n_items() > 0
                }));

            let filter = gtk::EveryFilter::new();
            filter.append(search_filter);
            filter.append(state_filter);

            let sorter = gtk::CustomSorter::new(|obj1, obj2| {
                let image1 = obj1.downcast_ref::<model::Image>().unwrap();
                let image2 = obj2.downcast_ref::<model::Image>().unwrap();

                if image1.repo_tags().len() == 0 {
                    if image2.repo_tags().len() == 0 {
                        image1.id().cmp(&image2.id()).into()
                    } else {
                        gtk::Ordering::Larger
                    }
                } else if image2.repo_tags().len() == 0 {
                    gtk::Ordering::Smaller
                } else {
                    image1.id().cmp(&image2.id()).into()
                }
            });

            self.filter.set(filter.upcast()).unwrap();
            self.sorter.set(sorter.upcast()).unwrap();
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ImagesPanel {}

    #[gtk::template_callbacks]
    impl ImagesPanel {
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

        #[template_callback]
        fn on_hide_intermediates_toggle_button_notify_active(&self) {
            self.update_filter(if self.hide_intermediates_toggle_button.is_active() {
                gtk::FilterChange::MoreStrict
            } else {
                gtk::FilterChange::LessStrict
            });
        }

        pub(super) fn set_image_list(&self, value: &model::ImageList) {
            let obj = &*self.obj();
            if obj.image_list().as_ref() == Some(value) {
                return;
            }

            value.connect_notify_local(
                Some("intermediates"),
                clone!(@weak obj => move |_ ,_| {
                    let imp = obj.imp();
                    imp.update_filter(gtk::FilterChange::Different);
                    imp.update_sorter();
                }),
            );

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

            self.list_box.set_visible(model.n_items() > 0);
            model.connect_items_changed(clone!(@weak obj => move |model, _, _, _| {
                obj.imp().list_box.set_visible(model.n_items() > 0);
            }));

            obj.action_set_enabled(ACTION_DELETE_SELECTION, false);
            value.connect_notify_local(
                Some("num-selected"),
                clone!(@weak obj => move |list, _| {
                    obj.action_set_enabled(ACTION_DELETE_SELECTION, list.num_selected() > 0);
                }),
            );

            self.image_list.set(Some(value));
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
    pub(crate) fn set_search_mode(&self, value: bool) {
        self.imp().search_bar.set_search_mode(value);
    }

    pub(crate) fn toggle_search_mode(&self) {
        self.set_search_mode(!self.imp().search_bar.is_search_mode());
    }

    pub(crate) fn show_download_page(&self) {
        if let Some(client) = self.client() {
            utils::show_dialog(
                self.upcast_ref(),
                view::ImagePullPage::from(&client).upcast_ref(),
            );
        }
    }

    pub(crate) fn show_build_page(&self) {
        if let Some(client) = self.client() {
            utils::show_dialog(
                self.upcast_ref(),
                view::ImageBuildPage::from(&client).upcast_ref(),
            );
        }
    }

    pub(crate) fn show_prune_page(&self) {
        if let Some(client) = self.client() {
            utils::show_dialog(
                self.upcast_ref(),
                view::ImagesPrunePage::from(&client).upcast_ref(),
            );
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

        let dialog = adw::MessageDialog::builder()
            .heading(gettext("Confirm Forced Deletion of Multiple Images"))
            .body(gettext(
                "There may be containers associated with those images, which will also be removed!",
            ))
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
                if let Some(list) = obj.image_list() {
                    list
                        .selected_items()
                        .iter().map(|obj| obj.downcast_ref::<model::Image>().unwrap())
                        .for_each(|image|
                    {
                        image.delete(clone!(@weak obj => move |image, result| {
                            if let Err(e) = result {
                                utils::show_error_toast(
                                    obj.upcast_ref(),
                                    // Translators: The first "{}" is a placeholder for the image id, the second is for an error message.
                                    &gettext!("Error on deleting image '{}'", image.id()),
                                    &e.to_string()
                                );
                            }
                        }));
                    });
                    list.set_selection_mode(false);
                }
            }),
        );

        dialog.present();
    }

    fn client(&self) -> Option<model::Client> {
        self.image_list()
            .as_ref()
            .and_then(model::ImageList::client)
    }
}
