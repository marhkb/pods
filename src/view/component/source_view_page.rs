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

pub(crate) enum Entity {
    Image(podman::api::Image),
    Container {
        container: podman::api::Container,
        mode: Mode,
    },
    Pod {
        pod: podman::api::Pod,
        mode: Mode,
    },
}

pub(crate) enum Mode {
    Inspect,
    Kube,
}

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/component/source-view-page.ui")]
    pub(crate) struct SourceViewPage {
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
    impl ObjectSubclass for SourceViewPage {
        const NAME: &'static str = "PdsSourceViewPage";
        type Type = super::SourceViewPage;
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

    impl ObjectImpl for SourceViewPage {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

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

            let adw_style_manager = adw::StyleManager::default();
            obj.on_notify_dark(&adw_style_manager);
            adw_style_manager.connect_dark_notify(clone!(@weak obj => move |style_manager| {
                obj.on_notify_dark(style_manager);
            }));
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for SourceViewPage {}
}

glib::wrapper! {
    pub(crate) struct SourceViewPage(ObjectSubclass<imp::SourceViewPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<Entity> for SourceViewPage {
    fn from(entity: Entity) -> Self {
        let obj: Self = glib::Object::builder::<Self>().build();
        let imp = obj.imp();

        imp.window_title.set_title(&match &entity {
            Entity::Image(_) => gettext("Image Inspection"),
            Entity::Container { mode, .. } => match mode {
                Mode::Inspect => gettext("Container Inspection"),
                Mode::Kube => gettext("Container Kube Generation"),
            },
            Entity::Pod { mode, .. } => match mode {
                Mode::Inspect => gettext("Pod Inspection"),
                Mode::Kube => gettext("Pod Kube Generation"),
            },
        });

        let language = match &entity {
            Entity::Image(_) => "json",
            Entity::Container { mode, .. } => match mode {
                Mode::Inspect => "json",
                Mode::Kube => "yaml",
            },
            Entity::Pod { mode, .. } => match mode {
                Mode::Inspect => "json",
                Mode::Kube => "yaml",
            },
        };

        match sourceview5::LanguageManager::default().language(language) {
            Some(lang) => imp.source_buffer.set_language(Some(&lang)),
            None => {
                log::warn!("Could not set language to '{language}'");
                utils::show_toast(&obj, &gettext!("Could not set language to '{}'", language));
            }
        }

        utils::do_async(
            async move {
                match entity {
                    Entity::Image(image) => image
                        .inspect()
                        .await
                        .map_err(anyhow::Error::from)
                        .and_then(|data| {
                            serde_json::to_string_pretty(&data).map_err(anyhow::Error::from)
                        }),
                    Entity::Container { container, mode } => match mode {
                        Mode::Inspect => container
                            .inspect()
                            .await
                            .map_err(anyhow::Error::from)
                            .and_then(|data| {
                                serde_json::to_string_pretty(&data).map_err(anyhow::Error::from)
                            }),
                        Mode::Kube => container
                            .generate_kube_yaml(false)
                            .await
                            .map_err(anyhow::Error::from),
                    },
                    Entity::Pod { pod, mode } => match mode {
                        Mode::Inspect => {
                            pod.inspect()
                                .await
                                .map_err(anyhow::Error::from)
                                .and_then(|data| {
                                    serde_json::to_string_pretty(&data).map_err(anyhow::Error::from)
                                })
                        }
                        Mode::Kube => pod
                            .generate_kube_yaml(false)
                            .await
                            .map_err(anyhow::Error::from),
                    },
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

impl SourceViewPage {
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
