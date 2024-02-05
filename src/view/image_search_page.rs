use std::cell::Cell;
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

const ACTION_SEARCH: &str = "image-search-page.search";
const ACTION_SELECT: &str = "image-search-page.select";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageSearchPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_search_page.ui")]
    pub(crate) struct ImageSearchPage {
        pub(super) search_results: OnceCell<gio::ListStore>,
        pub(super) search_abort_handle: RefCell<Option<future::AbortHandle>>,
        #[property(get, set, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set)]
        pub(super) show_cancel_button: Cell<bool>,
        #[property(get, set, construct)]
        pub(super) action_button_name: RefCell<String>,
        #[property(get, set, construct)]
        pub(super) top_level: OnceCell<bool>,
        #[template_child]
        pub(super) size_group: TemplateChild<gtk::SizeGroup>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) cancel_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
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
    impl ObjectSubclass for ImageSearchPage {
        const NAME: &'static str = "PdsImageSearchPage";
        type Type = super::ImageSearchPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.add_binding_action(gdk::Key::F, gdk::ModifierType::CONTROL_MASK, ACTION_SEARCH);
            klass.install_action(ACTION_SEARCH, None, |widget, _, _| {
                widget.grab_search_entry_focus();
            });

            klass.install_action(ACTION_SELECT, None, |widget, _, _| {
                widget.select();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageSearchPage {
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

            self.search_entry.set_key_capture_widget(Some(obj));

            let sort_list_model = gtk::SortListModel::new(
                Some(self.search_results().to_owned()),
                Some(
                    gtk::NumericSorter::builder()
                        .sort_order(gtk::SortType::Descending)
                        .expression(model::ImageSearchResponse::this_expression("stars"))
                        .build(),
                ),
            );

            self.selection.set_model(Some(&sort_list_model));

            self.list_view.remove_css_class("view");
        }

        fn dispose(&self) {
            if let Some(abort_handle) = self.search_abort_handle.take() {
                abort_handle.abort();
            }
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ImageSearchPage {}

    impl PreferencesGroupImpl for ImageSearchPage {}

    #[gtk::template_callbacks]
    impl ImageSearchPage {
        #[template_callback]
        fn on_notify_show_cancel_button(&self) {
            if self.obj().show_cancel_button() {
                self.size_group.add_widget(&self.cancel_button.get())
            } else {
                self.size_group.remove_widget(&self.cancel_button.get())
            }
        }

        #[template_callback]
        fn on_search_entry_search_changed(&self) {
            let obj = self.obj();

            if let Some(abort_handle) = self.search_abort_handle.take() {
                abort_handle.abort();
            }

            obj.action_set_enabled(ACTION_SELECT, false);

            self.search_results.get().unwrap().remove_all();

            let term = self.search_entry.text();
            if term.is_empty() {
                self.search_stack.set_visible_child_name("initial");
                return;
            }

            self.search_stack.set_visible_child_name("searching");

            let (abort_handle, abort_registration) = future::AbortHandle::new_pair();
            self.search_abort_handle.replace(Some(abort_handle));

            utils::do_async(
                {
                    let opts = podman::opts::ImageSearchOpts::builder()
                        .term(term.as_str())
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

                            if responses.is_empty() {
                                obj.action_set_enabled(ACTION_SELECT, false);

                                imp.search_stack.set_visible_child_name("nothing");
                                imp.no_results_status_page.set_title(&gettext!("No Results For {}", term));

                            } else {
                                obj.action_set_enabled(ACTION_SELECT, true);

                                responses.into_iter().for_each(|response| {
                                    imp.search_results()
                                        .append(&model::ImageSearchResponse::from(response));
                                });
                                imp.selection.set_selected(0);
                                imp.search_stack.set_visible_child_name("results");

                                glib::idle_add_local_once(clone!(@weak obj => move || {
                                    obj.imp().scrolled_window.emit_scroll_child(gtk::ScrollType::Start, false);
                                }));
                            }
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
        }

        #[template_callback]
        fn on_search_entry_key_pressed(
            &self,
            key: gdk::Key,
            _: u32,
            _: gdk::ModifierType,
            _: &gtk::EventControllerKey,
        ) -> glib::Propagation {
            if key == gdk::Key::Escape {
                let obj = &*self.obj();
                obj.activate_action(
                    if obj.top_level() {
                        "window.close"
                    } else {
                        "navigation.pop"
                    },
                    None,
                )
                .unwrap();
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
    }
}

glib::wrapper! {
    pub(crate) struct ImageSearchPage(ObjectSubclass<imp::ImageSearchPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ImageSearchPage {
    pub(crate) fn new(client: &model::Client, action_button_name: &str, top_level: bool) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("action-button-name", action_button_name)
            .property("top-level", top_level)
            .build()
    }

    pub(crate) fn grab_search_entry_focus(&self) {
        self.imp().search_entry.grab_focus();
    }

    pub(crate) fn select(&self) {
        let Some(client) = self.client() else {
            return;
        };

        let imp = self.imp();

        let Some(image) = imp
            .selection
            .selected_item()
            .and_then(|item| item.downcast::<model::ImageSearchResponse>().ok())
            .and_then(|image| image.name())
        else {
            return;
        };

        let page = view::RepoTagSelectionPage::new(&client, &image, &self.action_button_name());

        page.connect_image_selected(clone!(@weak self as obj => move |_, image| {
            obj.imp().navigation_view.pop();
            obj.emit_by_name::<()>("image-selected", &[image]);
        }));

        self.imp()
            .navigation_view
            .push(&adw::NavigationPage::new(&page, &gettext("Select Tag")));
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
