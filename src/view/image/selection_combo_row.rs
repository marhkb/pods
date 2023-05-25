use std::cell::Cell;

use adw::subclass::prelude::*;
use adw::traits::ActionRowExt;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_SELECT_IMAGE: &str = "container-creation-page.select-image";
const ACTION_SEARCH_IMAGE: &str = "container-creation-page.search-image";

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ImageSelectionRowMode")]
pub(crate) enum Mode {
    #[default]
    Unset,
    Local,
    Remote,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::SelectionComboRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image/selection-combo-row.ui")]
    pub(crate) struct SelectionComboRow {
        #[property(get, set, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, builder(Mode::default()))]
        pub(super) mode: Cell<Mode>,
        #[property(get, set = Self::set_image, construct, nullable)]
        pub(super) image: glib::WeakRef<model::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SelectionComboRow {
        const NAME: &'static str = "PdsImageSelectionComboRow";
        type Type = super::SelectionComboRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_SELECT_IMAGE, None, |widget, _, _| {
                widget.select_image();
            });

            klass.install_action(ACTION_SEARCH_IMAGE, None, |widget, _, _| {
                widget.search_image();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SelectionComboRow {
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

            Self::Type::this_expression("mode")
                .chain_closure::<String>(closure!(|_: Self::Type, mode: Mode| {
                    match mode {
                        Mode::Unset => gettext("Select Image"),
                        Mode::Local => gettext("Local Image"),
                        Mode::Remote => gettext("Remote Image"),
                    }
                }))
                .bind(obj, "title", Some(obj));
        }
    }

    impl WidgetImpl for SelectionComboRow {}
    impl ListBoxRowImpl for SelectionComboRow {}
    impl PreferencesRowImpl for SelectionComboRow {}
    impl ActionRowImpl for SelectionComboRow {}

    impl SelectionComboRow {
        fn set_image(&self, value: Option<&model::Image>) {
            let obj = &*self.obj();
            if obj.image().as_ref() == value {
                return;
            }

            self.image.set(value);

            if let Some(image) = value {
                obj.set_subtitle(
                    &image
                        .repo_tags()
                        .get(0)
                        .as_ref()
                        .map(model::RepoTag::full)
                        .unwrap_or_else(|| utils::format_id(&image.id())),
                );
                obj.set_mode(Mode::Local);
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct SelectionComboRow(ObjectSubclass<imp::SelectionComboRow>)
        @extends adw::ActionRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl SelectionComboRow {
    pub(crate) fn select_image(&self) {
        if let Some(client) = self.client() {
            let image_selection_page = view::ImagesSelectionPage::from(&client.image_list());
            image_selection_page.connect_image_selected(
                clone!(@weak self as obj => move |_, image| {
                    obj.set_image(Some(image));
                }),
            );
            utils::find_leaflet_overlay(self.upcast_ref())
                .show_details(image_selection_page.upcast_ref());
        }
    }

    pub(crate) fn search_image(&self) {
        if let Some(client) = self.client() {
            let image_search_page = view::ImageSearchPage::from(&client);
            image_search_page.connect_image_selected(clone!(@weak self as obj => move |_, image| {
                obj.set_image(Option::<model::Image>::None);
                obj.set_mode(Mode::Remote);
                obj.set_subtitle(&image);
            }));
            utils::find_leaflet_overlay(self.upcast_ref())
                .show_details(image_search_page.upcast_ref());
        }
    }
}
