use std::cell::OnceCell;
use std::cell::RefCell;
use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::*;
use futures::future;
use gettextrs::gettext;
use glib::clone;
use glib::Properties;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::glib::subclass::Signal;
use gtk::CompositeTemplate;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;

const ACTION_SELECT: &str = "repo-tag-selection-page.select";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::RepoTagSelectionPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/repo_tag_selection_page.ui")]
    pub(crate) struct RepoTagSelectionPage {
        pub(super) search_abort_handle: OnceCell<future::AbortHandle>,
        pub(super) search_results: OnceCell<gio::ListStore>,
        pub(super) filter: OnceCell<gtk::Filter>,
        #[property(get, set, construct)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, construct_only)]
        pub(super) image_name: OnceCell<String>,
        #[property(get, set, construct)]
        pub(super) action_button_name: RefCell<String>,
        #[template_child]
        pub(super) filter_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) title_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) filter_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) search_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) no_results_status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub(super) list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) selection: TemplateChild<gtk::SingleSelection>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RepoTagSelectionPage {
        const NAME: &'static str = "PdsRepoTagSelectionPage";
        type Type = super::RepoTagSelectionPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.add_binding(gdk::Key::F, gdk::ModifierType::CONTROL_MASK, |widget| {
                widget.enable_search_mode(true);
                glib::Propagation::Proceed
            });
            klass.add_binding(gdk::Key::Escape, gdk::ModifierType::empty(), |widget| {
                widget.enable_search_mode(false);
                glib::Propagation::Proceed
            });

            klass.install_action(ACTION_SELECT, None, |widget, _, _| {
                widget.select();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RepoTagSelectionPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("image-selected")
                    .param_types([String::static_type()])
                    .build()]
            })
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

            obj.action_set_enabled(ACTION_SELECT, false);

            let (abort_handle, abort_registration) = future::AbortHandle::new_pair();
            self.search_abort_handle.set(abort_handle).unwrap();

            utils::do_async(
                {
                    let opts = podman::opts::ImageSearchOpts::builder()
                        .term(self.image_name.get().unwrap())
                        .list_tags(true)
                        .limit(u32::MAX as usize)
                        .build();

                    let podman = obj.client().unwrap().podman();
                    async move {
                        future::Abortable::new(podman.images().search(&opts), abort_registration)
                            .await
                    }
                },
                clone!(@weak obj => move |result| if let Ok(responses) = result {
                    match responses {
                        Ok(responses) => {
                            let imp = obj.imp();

                            obj.action_set_enabled(ACTION_SELECT, true);

                            responses.into_iter().for_each(|response| {
                                obj
                                    .imp()
                                    .search_results()
                                    .append(&model::ImageSearchResponse::from(response));
                            });

                            imp.selection.set_selected(0);
                            imp.search_stack.set_visible_child_name("results");

                            glib::idle_add_local_once(clone!(@weak obj => move || {
                                obj.imp().scrolled_window.emit_scroll_child(gtk::ScrollType::Start, false);
                            }));
                        }
                        Err(e) => {
                            log::error!("Failed to search for images: {}", e);
                            utils::show_error_toast(
                                obj.upcast_ref(),
                                &gettext("Failed to search for images"),
                                &e.to_string());
                        }
                    }
                }),
            );

            self.filter_entry.set_key_capture_widget(Some(obj));

            let filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    let text = obj.imp().filter_entry.text();
                    let mut terms = text.split_ascii_whitespace();
                    let tag = item
                        .downcast_ref::<model::ImageSearchResponse>()
                        .unwrap()
                        .tag()
                        .unwrap();
                    terms.all(|term| tag.contains(&term.to_ascii_lowercase()))
                }));
            self.filter.set(filter.clone().upcast()).unwrap();
            let filter_list_model =
                gtk::FilterListModel::new(Some(self.search_results().to_owned()), Some(filter));

            let sort_list_model = gtk::SortListModel::new(
                Some(filter_list_model),
                Some(gtk::CustomSorter::new(|lhs, rhs| {
                    let lhs = lhs
                        .downcast_ref::<model::ImageSearchResponse>()
                        .unwrap()
                        .tag()
                        .unwrap();

                    if lhs == "latest" {
                        return gtk::Ordering::Smaller;
                    }

                    let rhs = rhs
                        .downcast_ref::<model::ImageSearchResponse>()
                        .unwrap()
                        .tag()
                        .unwrap();

                    if rhs == "latest" {
                        return gtk::Ordering::Larger;
                    }

                    gtk::Ordering::Equal
                })),
            );

            self.list_view.remove_css_class("view");

            self.selection.set_model(Some(&sort_list_model));
        }

        fn dispose(&self) {
            self.search_abort_handle.get().unwrap().abort();
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for RepoTagSelectionPage {}

    impl PreferencesGroupImpl for RepoTagSelectionPage {}

    #[gtk::template_callbacks]
    impl RepoTagSelectionPage {
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
            }

            glib::Propagation::Proceed
        }

        #[template_callback]
        fn on_signal_list_item_factory_setup(&self, list_item: &gtk::ListItem) {
            list_item.set_child(Some(&view::ImageSearchResponseRow::default()));
        }

        #[template_callback]
        fn on_signal_list_item_factory_bind(&self, list_item: &gtk::ListItem) {
            let response = list_item
                .item()
                .and_downcast::<model::ImageSearchResponse>()
                .unwrap();

            list_item
                .child()
                .and_downcast::<view::ImageSearchResponseRow>()
                .unwrap()
                .set_image_search_response(Some(response));
        }

        #[template_callback]
        fn on_image_activated(&self, _: u32) {
            self.obj().activate_action(ACTION_SELECT, None).unwrap();
        }

        pub(super) fn search_results(&self) -> &gio::ListStore {
            self.search_results
                .get_or_init(gio::ListStore::new::<model::ImageSearchResponse>)
        }

        pub(super) fn update_filter(&self, change: gtk::FilterChange) {
            self.filter.get().unwrap().changed(change);
        }
    }
}

glib::wrapper! {
    pub(crate) struct RepoTagSelectionPage(ObjectSubclass<imp::RepoTagSelectionPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RepoTagSelectionPage {
    pub(crate) fn new(client: &model::Client, image_name: &str, action_button_name: &str) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("image-name", image_name)
            .property("action-button-name", action_button_name)
            .build()
    }

    pub(crate) fn enable_search_mode(&self, enable: bool) {
        let imp = self.imp();

        if !enable && !imp.filter_button.is_active() {
            self.activate_action("navigation.pop", None).unwrap();
        } else {
            imp.filter_button.set_active(enable);
            if !enable {
                imp.update_filter(gtk::FilterChange::LessStrict);
            }
        }
    }

    pub(crate) fn select(&self) {
        if let Some(image) = self.selected_image() {
            self.emit_by_name::<()>("image-selected", &[&image]);
        }
    }

    pub(crate) fn selected_image(&self) -> Option<String> {
        self.imp()
            .selection
            .selected_item()
            .map(|item| item.downcast::<model::ImageSearchResponse>().unwrap())
            .map(|image| format!("{}:{}", image.name().unwrap(), image.tag().unwrap()))
    }

    pub(crate) fn connect_image_selected<F: Fn(&Self, &String) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("image-selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let image = values[1].get::<String>().unwrap();
            f(&obj, &image);

            None
        })
    }
}
