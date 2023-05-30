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

pub(crate) const ACTION_PULL_IMAGE: &str = "images-panel.pull-image";
const ACTION_BUILD_IMAGE: &str = "images-panel.build-image";
const ACTION_PRUNE_UNUSED_IMAGES: &str = "images-panel.prune-unused-images";
const ACTION_SHOW_ADD_IMAGE_MENU: &str = "images-panel.show-add-image-menu";
const ACTION_DELETE_SELECTION: &str = "images-panel.delete-selection";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImagesPanel)]
    #[template(file = "images_panel.ui")]
    pub(crate) struct ImagesPanel {
        pub(super) settings: utils::PodsSettings,
        pub(super) properties_filter: UnsyncOnceCell<gtk::Filter>,
        pub(super) sorter: UnsyncOnceCell<gtk::Sorter>,
        #[property(get, set = Self::set_image_list, nullable)]
        pub(super) image_list: glib::WeakRef<model::ImageList>,
        #[template_child]
        pub(super) add_image_row: TemplateChild<gtk::ListBoxRow>,
        #[template_child]
        pub(super) popover_menu: TemplateChild<gtk::PopoverMenu>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) images_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) header_suffix_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) show_intermediates_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) header_suffix_button_box: TemplateChild<gtk::Box>,
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

            klass.install_action(ACTION_SHOW_ADD_IMAGE_MENU, None, |widget, _, _| {
                widget.show_add_image_menu();
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

            self.popover_menu.set_parent(&*self.add_image_row);

            self.settings
                .bind(
                    "show-intermediate-images",
                    &*self.show_intermediates_switch,
                    "active",
                )
                .build();

            let image_list_expr = Self::Type::this_expression("image-list");
            let image_list_len_expr = image_list_expr.chain_property::<model::ImageList>("len");
            let is_selection_mode_expr = image_list_expr
                .chain_property::<model::ImageList>("selection-mode")
                .chain_closure::<bool>(closure!(|_: Self::Type, selection_mode: bool| {
                    !selection_mode
                }));

            image_list_len_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, len: u32| len > 0))
                .bind(&*self.header_suffix_box, "visible", Some(obj));

            is_selection_mode_expr.bind(&*self.header_suffix_button_box, "visible", Some(obj));
            is_selection_mode_expr.bind(&*self.add_image_row, "visible", Some(obj));

            image_list_len_expr.watch(
                Some(obj),
                clone!(@weak obj => move || {
                    let list = obj.image_list().unwrap();
                    if list.is_selection_mode() && list.len() == 0 {
                        list.set_selection_mode(false);
                        obj.emit_by_name::<()>("exit-selection-mode", &[]);
                    }
                }),
            );

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
            .bind(&*self.main_stack, "visible-child-name", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [image_list_expr, image_list_len_expr],
                closure!(|_: Self::Type, list: Option<model::ImageList>, len: u32| {
                    match list {
                        Some(list) => {
                            if len == 0 {
                                gettext("No images found")
                            } else if len == 1 {
                                if list.intermediates() == 0 {
                                    gettext("1 image, used")
                                } else {
                                    gettext("1 image, unused")
                                }
                            } else {
                                ngettext!(
                                    // Translators: There's a wide space (U+2002) between ", {}".
                                    "{} image total, {} {} unused, {}",
                                    "{} images total, {} {} unused, {}",
                                    len,
                                    len,
                                    glib::format_size(list.total_size()),
                                    list.intermediates(),
                                    glib::format_size(list.unused_size()),
                                )
                            }
                        }
                        None => gettext("No images found"),
                    }
                }),
            )
            .bind(&*self.images_group, "description", Some(obj));

            let properties_filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    obj.imp().show_intermediates_switch.is_active()
                    || item
                        .downcast_ref::<model::Image>()
                        .unwrap()
                        .repo_tags()
                        .n_items() > 0
                }));

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

            self.properties_filter
                .set(properties_filter.upcast())
                .unwrap();
            self.sorter.set(sorter.upcast()).unwrap();

            self.show_intermediates_switch.connect_active_notify(
                clone!(@weak obj => move |switch| {
                    obj.update_properties_filter(
                        if switch.is_active() {
                            gtk::FilterChange::LessStrict
                        } else {
                            gtk::FilterChange::MoreStrict
                        }
                    );
                }),
            );
        }

        fn dispose(&self) {
            self.popover_menu.unparent();
            self.main_stack.unparent();
        }
    }

    impl WidgetImpl for ImagesPanel {}

    impl ImagesPanel {
        pub(super) fn set_image_list(&self, value: &model::ImageList) {
            let obj = &*self.obj();
            if obj.image_list().as_ref() == Some(value) {
                return;
            }

            value.connect_notify_local(
                Some("intermediates"),
                clone!(@weak obj => move |_ ,_| {
                    obj.update_properties_filter(gtk::FilterChange::Different);
                    obj.update_sorter();
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
                view::ImageRow::from(item.downcast_ref().unwrap()).upcast()
            });
            self.list_box.append(&*self.add_image_row);

            obj.action_set_enabled(ACTION_DELETE_SELECTION, false);
            value.connect_notify_local(
                Some("num-selected"),
                clone!(@weak obj => move |list, _| {
                    obj.action_set_enabled(ACTION_DELETE_SELECTION, list.num_selected() > 0);
                }),
            );

            self.image_list.set(Some(value));
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
    pub(crate) fn action_pull_image() -> &'static str {
        ACTION_PULL_IMAGE
    }

    pub(crate) fn update_properties_filter(&self, filter_change: gtk::FilterChange) {
        self.imp()
            .properties_filter
            .get()
            .unwrap()
            .changed(filter_change);
    }

    fn show_download_page(&self) {
        if let Some(client) = self.client() {
            utils::show_dialog(
                self.upcast_ref(),
                view::ImagePullPage::from(&client).upcast_ref(),
            );
        }
    }

    fn show_build_page(&self) {
        if let Some(client) = self.client() {
            utils::show_dialog(
                self.upcast_ref(),
                view::ImageBuildPage::from(&client).upcast_ref(),
            );
        }
    }

    fn show_prune_page(&self) {
        if let Some(client) = self.client() {
            utils::show_dialog(
                self.upcast_ref(),
                view::ImagesPrunePage::from(&client).upcast_ref(),
            );
        }
    }

    fn show_add_image_menu(&self) {
        self.imp().popover_menu.popup();
    }

    fn delete_selection(&self) {
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
                    obj.emit_by_name::<()>("exit-selection-mode", &[]);
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

    fn update_sorter(&self) {
        self.imp()
            .sorter
            .get()
            .unwrap()
            .changed(gtk::SorterChange::Different);
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
