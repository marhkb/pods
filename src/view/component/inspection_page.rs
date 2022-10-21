use gettextrs::gettext;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use sourceview5::traits::BufferExt;

use crate::podman;
use crate::utils;
use crate::view;

const ACTION_TOGGLE_SEARCH: &str = "inspection-page.toggle-search";

pub(crate) enum Inspectable {
    Image(podman::api::Image),
    Container(podman::api::Container),
    Pod(podman::api::Pod),
}

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/component/inspection-page.ui")]
    pub(crate) struct InspectionPage {
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub(super) search_widget: TemplateChild<view::SourceViewSearchWidget>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) source_view: TemplateChild<sourceview5::View>,
        #[template_child]
        pub(super) source_buffer: TemplateChild<sourceview5::Buffer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InspectionPage {
        const NAME: &'static str = "PdsInspectionPage";
        type Type = super::InspectionPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.add_binding_action(
                gdk::Key::F,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_TOGGLE_SEARCH,
                None,
            );
            klass.install_action(ACTION_TOGGLE_SEARCH, None, |widget, _, _| {
                widget.toggle_search();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for InspectionPage {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            self.search_bar.connect_search_mode_enabled_notify(
                clone!(@weak obj => move |search_bar| {
                    let search_entry = &*obj.imp().search_widget;
                    if search_bar.is_search_mode() {
                        search_entry.grab_focus();
                    } else {
                        search_entry.set_text("");
                    }
                }),
            );

            self.search_button
                .bind_property("active", &*self.search_bar, "search-mode-enabled")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.search_widget.set_source_view(Some(&*self.source_view));

            match sourceview5::LanguageManager::default().language("json") {
                Some(lang) => self.source_buffer.set_language(Some(&lang)),
                None => {
                    log::warn!("Could not set language to 'json'");
                    utils::show_toast(obj, &gettext("Could not set language to 'json'"));
                }
            }

            let adw_style_manager = adw::StyleManager::default();
            obj.on_notify_dark(&adw_style_manager);
            adw_style_manager.connect_dark_notify(clone!(@weak obj => move |style_manager| {
                obj.on_notify_dark(style_manager);
            }));
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.instance()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for InspectionPage {}
}

glib::wrapper! {
    pub(crate) struct InspectionPage(ObjectSubclass<imp::InspectionPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<Inspectable> for InspectionPage {
    fn from(inspectabele: Inspectable) -> Self {
        let obj: Self = glib::Object::builder::<Self>().build();

        obj.imp().window_title.set_title(&match &inspectabele {
            Inspectable::Image(_) => gettext("Image Inspection"),
            Inspectable::Container(_) => gettext("Container Inspection"),
            Inspectable::Pod(_) => gettext("Pod Inspection"),
        });

        utils::do_async(
            async move {
                match inspectabele {
                    Inspectable::Image(image) => image
                        .inspect()
                        .await
                        .map_err(anyhow::Error::from)
                        .and_then(|data| {
                            serde_json::to_string_pretty(&data).map_err(anyhow::Error::from)
                        }),
                    Inspectable::Container(container) => container
                        .inspect()
                        .await
                        .map_err(anyhow::Error::from)
                        .and_then(|data| {
                            serde_json::to_string_pretty(&data).map_err(anyhow::Error::from)
                        }),
                    Inspectable::Pod(pod) => pod
                        .inspect()
                        .await
                        .map_err(anyhow::Error::from)
                        .and_then(|data| {
                            serde_json::to_string_pretty(&data).map_err(anyhow::Error::from)
                        }),
                }
            },
            clone!(@weak obj => move |result| {
                let imp = obj.imp();
                match result {
                    Ok(text) =>  {
                        imp.source_buffer.set_text(&text);
                        imp.stack.set_visible_child_name("loaded");
                    }
                    Err(e) => {
                        imp.spinner.set_spinning(false);
                        utils::show_error_toast(
                            &obj,
                            &gettext("Inspection error"),
                            &e.to_string()
                        );
                    }
                }
            }),
        );

        obj
    }
}

impl InspectionPage {
    pub(crate) fn toggle_search(&self) {
        let imp = self.imp();
        imp.search_bar
            .set_search_mode(!imp.search_bar.is_search_mode());
    }

    fn on_notify_dark(&self, style_manager: &adw::StyleManager) {
        self.imp().source_buffer.set_style_scheme(
            sourceview5::StyleSchemeManager::default()
                .scheme(if style_manager.is_dark() {
                    "Adwaita-dark"
                } else {
                    "Adwaita"
                })
                .as_ref(),
        );
    }
}
