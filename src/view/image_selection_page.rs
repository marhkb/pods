use std::cell::OnceCell;
use std::cmp::Ordering;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::clone;
use glib::once_cell::sync::Lazy as SyncLazy;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::glib::subclass::Signal;
use gtk::pango;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

const ACTION_FILTER: &str = "image-selection-page.filter";
const ACTION_SELECT: &str = "image-selection-page.select";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageSelectionPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_selection_page.ui")]
    pub(crate) struct ImageSelectionPage {
        pub(super) filter: OnceCell<gtk::Filter>,
        #[property(get, set = Self::set_image_list, nullable)]
        pub(super) image_list: glib::WeakRef<model::ImageList>,
        #[template_child]
        pub(super) filter_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) title_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) filter_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) select_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) selection: TemplateChild<gtk::SingleSelection>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageSelectionPage {
        const NAME: &'static str = "PdsImageSelectionPage";
        type Type = super::ImageSelectionPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_FILTER, Some("b"), |widget, _, data| {
                widget.enable_search_mode(data.unwrap().get().unwrap());
            });
            klass.add_binding_action(
                gdk::Key::F,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_FILTER,
                Some(&true.to_variant()),
            );
            klass.add_binding_action(
                gdk::Key::Escape,
                gdk::ModifierType::empty(),
                ACTION_FILTER,
                Some(&false.to_variant()),
            );

            klass.install_action(ACTION_SELECT, None, |widget, _, _| {
                widget.select();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageSelectionPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: SyncLazy<Vec<Signal>> = SyncLazy::new(|| {
                vec![Signal::builder("image-selected")
                    .param_types([model::Image::static_type()])
                    .build()]
            });
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

            self.filter_entry.set_key_capture_widget(Some(obj));

            let filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    let term = obj.imp().filter_entry.text().to_lowercase();

                    let image = item.downcast_ref::<model::Image>().unwrap();
                    image
                        .repo_tags()
                        .get(0)
                        .map(|repo_tag| repo_tag.full().contains(&term))
                        .unwrap_or_else(|| image.id().contains(&term))
                }));
            self.filter.set(filter.upcast()).unwrap();
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ImageSelectionPage {}

    #[gtk::template_callbacks]
    impl ImageSelectionPage {
        #[template_callback]
        fn on_filter_button_toggled(&self) {
            if self.filter_button.is_active() {
                self.filter_entry.grab_focus();
                self.title_stack.set_visible_child(&self.filter_entry.get());
            } else {
                self.filter_entry.set_text("");
                self.title_stack.set_visible_child_name("title");
            }
        }

        #[template_callback]
        fn on_filter_started(&self) {
            self.filter_button.set_active(true)
        }

        #[template_callback]
        fn on_filter_changed(&self) {
            self.update_filter(gtk::FilterChange::Different);
        }

        #[template_callback]
        fn on_filter_key_pressed(
            &self,
            key: gdk::Key,
            _: u32,
            _: gdk::ModifierType,
            _: &gtk::EventControllerKey,
        ) -> glib::Propagation {
            if key == gdk::Key::Escape {
                self.obj().enable_search_mode(false);
            } else if key == gdk::Key::KP_Enter {
                self.obj().activate_action(ACTION_SELECT, None).unwrap();
            }

            glib::Propagation::Proceed
        }

        #[template_callback]
        fn on_signal_list_item_factory_setup(&self, list_item: &gtk::ListItem) {
            let label = gtk::Label::builder()
                .margin_top(9)
                .margin_end(12)
                .margin_bottom(9)
                .margin_start(12)
                .xalign(0.0)
                .wrap(true)
                .wrap_mode(pango::WrapMode::WordChar)
                .build();

            list_item.set_child(Some(&label));
        }

        #[template_callback]
        fn on_signal_list_item_factory_bind(&self, list_item: &gtk::ListItem) {
            let image = list_item.item().and_downcast::<model::Image>().unwrap();
            let repo_tag = image.repo_tags().get(0);

            let label = list_item.child().and_downcast::<gtk::Label>().unwrap();
            label.set_label(
                &repo_tag
                    .as_ref()
                    .map(|repo_tag| repo_tag.full())
                    .unwrap_or_else(|| image.id()),
            );
            match image.repo_tags().get(0) {
                Some(_) => {
                    label.remove_css_class("dim-label");
                    label.remove_css_class("numeric");
                }
                None => {
                    label.add_css_class("dim-label");
                    label.add_css_class("numeric");
                }
            }
        }

        #[template_callback]
        fn on_image_selected(&self) {
            self.obj()
                .action_set_enabled(ACTION_SELECT, self.selection.selected_item().is_some());
        }

        #[template_callback]
        fn on_image_activated(&self, _: u32) {
            self.obj().activate_action(ACTION_SELECT, None).unwrap();
        }

        pub(super) fn set_image_list(&self, value: Option<&model::ImageList>) {
            let obj = &*self.obj();
            if obj.image_list().as_ref() == value {
                return;
            }

            if let Some(image_list) = value {
                let model = gtk::FilterListModel::new(
                    Some(image_list.to_owned()),
                    self.filter.get().cloned(),
                );

                let model = gtk::SortListModel::new(
                    Some(model),
                    Some(gtk::CustomSorter::new(|item1, item2| {
                        let image1 = item1.downcast_ref::<model::Image>().unwrap();
                        let image2 = item2.downcast_ref::<model::Image>().unwrap();

                        match image1.repo_tags().get(0) {
                            Some(repo_tag1) => match image2.repo_tags().get(0) {
                                Some(repo_tag2) => repo_tag1.full().cmp(&repo_tag2.full()),
                                _ => Ordering::Less,
                            },
                            _ if image2.repo_tags().get(0).is_some() => Ordering::Greater,
                            _ => image1.id().cmp(&image2.id()),
                        }
                        .into()
                    })),
                );

                let model = gtk::SingleSelection::new(Some(model));

                model.connect_selected_item_notify(clone!(@weak obj => move |selection| {
                    obj.action_set_enabled(ACTION_SELECT, selection.selected_item().is_some());
                }));

                self.selection.set_model(Some(&model));

                obj.action_set_enabled(ACTION_SELECT, self.selection.selected_item().is_some());
            }

            self.image_list.set(value);
        }

        pub(super) fn update_filter(&self, change: gtk::FilterChange) {
            self.filter.get().unwrap().changed(change);
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageSelectionPage(ObjectSubclass<imp::ImageSelectionPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::ImageList> for ImageSelectionPage {
    fn from(image_list: &model::ImageList) -> Self {
        glib::Object::builder()
            .property("image-list", image_list)
            .build()
    }
}

impl ImageSelectionPage {
    pub(crate) fn enable_search_mode(&self, enable: bool) {
        let imp = self.imp();

        if !enable && !imp.filter_button.is_active() {
            utils::navigation_view(self.upcast_ref()).pop();
        } else {
            imp.filter_button.set_active(enable);
            if !enable {
                imp.update_filter(gtk::FilterChange::LessStrict);
            }
        }
    }

    pub(crate) fn selected_image(&self) -> Option<model::Image> {
        self.imp()
            .selection
            .selected_item()
            .and_then(|item| item.downcast().ok())
    }

    pub(crate) fn select(&self) {
        if let Some(image) = self.selected_image() {
            self.emit_by_name::<()>("image-selected", &[&image]);

            utils::navigation_view(self.upcast_ref()).pop();
        }
    }

    pub(crate) fn connect_image_selected<F: Fn(&Self, model::Image) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("image-selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let image = values[1].get::<model::Image>().unwrap();
            f(&obj, image);

            None
        })
    }
}
