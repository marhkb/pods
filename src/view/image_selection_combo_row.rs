use std::cell::Cell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_SELECT_IMAGE: &str = "container-creation-page.select-image";
const ACTION_SEARCH_IMAGE: &str = "container-creation-page.search-image";

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ImageSelectionMode")]
pub(crate) enum ImageSelectionMode {
    #[default]
    Unset,
    Local,
    Remote,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageSelectionComboRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_selection_combo_row.ui")]
    pub(crate) struct ImageSelectionComboRow {
        #[property(get, set, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, builder(ImageSelectionMode::default()))]
        pub(super) mode: Cell<ImageSelectionMode>,
        #[property(get, set = Self::set_image, construct, nullable)]
        pub(super) image: glib::WeakRef<model::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageSelectionComboRow {
        const NAME: &'static str = "PdsImageSelectionComboRow";
        type Type = super::ImageSelectionComboRow;
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

    impl ObjectImpl for ImageSelectionComboRow {
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
                .chain_closure::<String>(closure!(|_: Self::Type, mode: ImageSelectionMode| {
                    match mode {
                        ImageSelectionMode::Unset => gettext("Select Image"),
                        ImageSelectionMode::Local => gettext("Local Image"),
                        ImageSelectionMode::Remote => gettext("Remote Image"),
                    }
                }))
                .bind(obj, "title", Some(obj));
        }
    }

    impl WidgetImpl for ImageSelectionComboRow {}
    impl ListBoxRowImpl for ImageSelectionComboRow {}
    impl PreferencesRowImpl for ImageSelectionComboRow {}
    impl ActionRowImpl for ImageSelectionComboRow {}

    impl ImageSelectionComboRow {
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
                obj.set_mode(ImageSelectionMode::Local);
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageSelectionComboRow(ObjectSubclass<imp::ImageSelectionComboRow>)
        @extends adw::ActionRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl ImageSelectionComboRow {
    pub(crate) fn select_image(&self) {
        if let Some(client) = self.client() {
            let image_selection_page = view::ImageSelectionPage::from(&client.image_list());
            image_selection_page.connect_image_selected(clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, image| {
                    obj.set_image(Some(image));
                }
            ));
            utils::navigation_view(self).push(
                &adw::NavigationPage::builder()
                    .child(&image_selection_page)
                    .build(),
            );
        }
    }

    pub(crate) fn search_image(&self) {
        if let Some(client) = self.client() {
            let image_search_page = view::ImageSearchPage::new(&client, &gettext("Select"), false);

            image_search_page.connect_image_selected(clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, image| {
                    obj.activate_action("navigation.pop", None).unwrap();

                    obj.set_image(Option::<model::Image>::None);
                    obj.set_mode(ImageSelectionMode::Remote);
                    obj.set_subtitle(image);
                }
            ));
            utils::navigation_view(self).push(
                &adw::NavigationPage::builder()
                    .child(&image_search_page)
                    .build(),
            );
        }
    }
}
