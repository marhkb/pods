use adw::prelude::MessageDialogExtManual;
use adw::traits::BinExt;
use adw::traits::MessageDialogExt;
use gettextrs::gettext;
use gettextrs::ngettext;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::subclass::Signal;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;

use crate::model;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

const ACTION_PULL_IMAGE: &str = "images-panel.pull-image";
const ACTION_BUILD_IMAGE: &str = "images-panel.build-image";
const ACTION_PRUNE_UNUSED_IMAGES: &str = "images-panel.prune-unused-images";
const ACTION_SHOW_ADD_IMAGE_MENU: &str = "images-panel.show-add-image-menu";
const ACTION_DELETE_SELECTION: &str = "images-panel.delete-selection";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/images/panel.ui")]
    pub(crate) struct Panel {
        pub(super) settings: utils::PodsSettings,
        pub(super) image_list: glib::WeakRef<model::ImageList>,
        pub(super) properties_filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
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
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Panel {
        const NAME: &'static str = "PdsImagesPanel";
        type Type = super::Panel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_PULL_IMAGE,
                None,
            );
            klass.install_action(ACTION_PULL_IMAGE, None, move |widget, _, _| {
                widget.show_download_page();
            });

            klass.install_action(ACTION_BUILD_IMAGE, None, move |widget, _, _| {
                widget.show_build_page();
            });

            klass.install_action(ACTION_PRUNE_UNUSED_IMAGES, None, move |widget, _, _| {
                widget.show_prune_page();
            });

            klass.install_action(ACTION_SHOW_ADD_IMAGE_MENU, None, move |widget, _, _| {
                widget.show_add_image_menu();
            });

            klass.install_action(ACTION_DELETE_SELECTION, None, move |widget, _, _| {
                widget.delete_selection();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Panel {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("exit-selection-mode").build()]);
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::ImageList>("image-list")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "image-list" => self.obj().set_image_list(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image-list" => self.obj().image_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            self.popover_menu.set_parent(&*self.add_image_row);

            self.settings.connect_changed(
                Some("show-intermediate-images"),
                clone!(@weak obj => move |_, _| obj.update_properties_filter()),
            );
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

            is_selection_mode_expr.bind(&*self.menu_button, "visible", Some(obj));
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
                &[
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
                &[image_list_expr, image_list_len_expr],
                closure!(|_: Self::Type, list: Option<model::ImageList>, _: u32| {
                    match list {
                        Some(list) => {
                            let len = list.n_items();

                            if len == 0 {
                                gettext("No images found")
                            } else if len == 1 {
                                if list.num_unused_images() == 0 {
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
                                    list.num_unused_images(),
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

            obj.connect_notify_local(
                Some("show-intermediates"),
                clone!(@weak obj => move |_ ,_| obj.update_properties_filter()),
            );

            let sorter = gtk::CustomSorter::new(|obj1, obj2| {
                let image1 = obj1.downcast_ref::<model::Image>().unwrap();
                let image2 = obj2.downcast_ref::<model::Image>().unwrap();

                if image1.repo_tags().n_items() == 0 {
                    if image2.repo_tags().n_items() == 0 {
                        image1.id().cmp(image2.id()).into()
                    } else {
                        gtk::Ordering::Larger
                    }
                } else if image2.repo_tags().n_items() == 0 {
                    gtk::Ordering::Smaller
                } else {
                    image1
                        .repo_tags()
                        .string(0)
                        .cmp(&image2.repo_tags().string(0))
                        .into()
                }
            });

            self.properties_filter
                .set(properties_filter.upcast())
                .unwrap();
            self.sorter.set(sorter.upcast()).unwrap();
        }

        fn dispose(&self) {
            self.popover_menu.unparent();
            self.main_stack.unparent();
        }
    }

    impl WidgetImpl for Panel {}
}

glib::wrapper! {
    pub(crate) struct Panel(ObjectSubclass<imp::Panel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Panel {
    fn default() -> Self {
        glib::Object::builder::<Self>().build()
    }
}

impl Panel {
    pub(crate) fn image_list(&self) -> Option<model::ImageList> {
        self.imp().image_list.upgrade()
    }

    pub(crate) fn set_image_list(&self, value: &model::ImageList) {
        if self.image_list().as_ref() == Some(value) {
            return;
        }

        // TODO: For multi-client: Figure out whether signal handlers need to be disconnected.
        let imp = self.imp();

        let model = gtk::SortListModel::new(
            Some(&gtk::FilterListModel::new(
                Some(value),
                imp.properties_filter.get(),
            )),
            imp.sorter.get(),
        );

        imp.list_box.bind_model(Some(&model), |item| {
            view::ImageRow::from(item.downcast_ref().unwrap()).upcast()
        });
        imp.list_box.append(&*imp.add_image_row);

        self.action_set_enabled(ACTION_DELETE_SELECTION, false);
        value.connect_notify_local(
            Some("num-selected"),
            clone!(@weak self as obj => move |list, _| {
                obj.action_set_enabled(ACTION_DELETE_SELECTION, list.num_selected() > 0);
            }),
        );

        imp.image_list.set(Some(value));
        self.notify("image-list");
    }

    pub(crate) fn update_properties_filter(&self) {
        self.imp()
            .properties_filter
            .get()
            .unwrap()
            .changed(gtk::FilterChange::Different);
    }

    fn show_download_page(&self) {
        let leaflet_overlay = utils::find_leaflet_overlay(self);

        if leaflet_overlay.child().is_none() {
            leaflet_overlay.show_details(&view::ImagePullPage::from(self.client().as_ref()));
        }
    }

    fn show_build_page(&self) {
        let leaflet_overlay = utils::find_leaflet_overlay(self);

        if leaflet_overlay.child().is_none() {
            leaflet_overlay.show_details(&view::ImageBuildPage::from(self.client().as_ref()));
        }
    }

    fn show_prune_page(&self) {
        let leaflet_overlay = utils::find_leaflet_overlay(self);

        if leaflet_overlay.child().is_none() {
            leaflet_overlay.show_details(&view::ImagesPrunePage::from(self.client().as_ref()));
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
            .heading(&gettext("Confirm Forced Deletion of Multiple Images"))
            .body(&gettext(
                "There may be containers associated with those images, which will also be removed!",
            ))
            .modal(true)
            .transient_for(&utils::root(self))
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
                                    &obj,
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
