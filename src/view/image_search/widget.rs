use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::ComboRowExt;
use futures::future;
use gettextrs::gettext;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::model;
use crate::podman;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image-search/widget.ui")]
    pub(crate) struct Widget {
        pub(super) client: glib::WeakRef<model::Client>,
        pub(super) search_results: gio::ListStore,
        pub(super) selection: OnceCell<gtk::SingleSelection>,
        pub(super) search_abort_handle: RefCell<Option<future::AbortHandle>>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) search_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) registries_combo_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) search_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) no_results_status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) search_result_list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) tag_entry_row: TemplateChild<adw::EntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Widget {
        const NAME: &'static str = "PdsImageSearchWidget";
        type Type = super::Widget;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Widget {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Client>("client")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecObject::builder::<model::ImageSearchResponse>("selected-image")
                        .read_only()
                        .build(),
                    glib::ParamSpecString::builder("tag")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecString::builder("default-tag")
                        .default_value(Some("latest"))
                        .read_only()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "client" => obj.set_client(value.get().unwrap()),
                "tag" => obj.set_tag(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "client" => obj.client().to_value(),
                "selected-image" => obj.selected_image().to_value(),
                "tag" => obj.tag().to_value(),
                "default-tag" => obj.default_tag().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            self.search_entry_row
                .connect_changed(clone!(@weak obj => move |_| obj.search()));

            self.registries_combo_row
                .set_expression(Some(&model::Registry::this_expression("name")));

            let filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    let imp = obj.imp();

                    imp.registries_combo_row.selected() == 0
                        || Some(
                            imp.registries_combo_row
                                .selected_item()
                                .unwrap()
                                .downcast::<model::Registry>()
                                .unwrap()
                                .name()
                                .as_str(),
                        ) == item
                            .downcast_ref::<model::ImageSearchResponse>()
                            .unwrap()
                            .index()
                }));

            self.registries_combo_row.connect_selected_item_notify(
                clone!(@weak filter => move |_| filter.changed(gtk::FilterChange::Different)),
            );

            let selection = gtk::SingleSelection::new(Some(gtk::FilterListModel::new(
                Some(self.search_results.clone()),
                Some(filter),
            )));

            selection.connect_selected_item_notify(clone!(@weak obj => move |_| {
                obj.notify("selected-image");
            }));

            self.search_result_list_view.set_model(Some(&selection));

            self.selection.set(selection).unwrap();

            self.tag_entry_row.connect_notify_local(
                None,
                clone!(@weak obj => move |_, pspec| {
                    match pspec.name() {
                        "tag" | "default-tag" => obj.notify(pspec.name()),
                        _ => {}
                    }
                }),
            );
        }

        fn dispose(&self) {
            self.stack.unparent();
        }
    }

    impl WidgetImpl for Widget {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            glib::idle_add_local(
                clone!(@weak widget => @default-return glib::Continue(false), move || {
                    widget.imp().search_entry_row.grab_focus();
                    glib::Continue(false)
                }),
            );
        }
    }

    impl PreferencesGroupImpl for Widget {}
}

glib::wrapper! {
    pub(crate) struct Widget(ObjectSubclass<imp::Widget>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Widget {
    pub(crate) fn action_select() -> &'static str {
        "image-search-widget.select"
    }

    pub(crate) fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    pub(crate) fn set_client(&self, client: Option<&model::Client>) {
        if self.client().as_ref() == client {
            return;
        }

        if let Some(client) = client {
            utils::do_async(
                {
                    let podman = client.podman().clone();
                    async move { podman.info().await }
                },
                clone!(@weak self as obj => move |result| match result {
                    Ok(info) => {
                        let imp = obj.imp();
                        match info.registries.unwrap().get("search") {
                            Some(search) => {
                                let model = gio::ListStore::new(model::Registry::static_type());
                                model.append(&model::Registry::from(gettext("All registries").as_str()));
                                search
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .for_each(|name| {
                                        model.append(&model::Registry::from(name.as_str().unwrap()))
                                    });

                                imp.registries_combo_row.set_model(Some(&model));
                                imp.stack.set_visible_child_name("search");
                            }
                            None => imp.stack.set_visible_child_name("no-registries"),
                        }
                    },
                    Err(e) => {
                        log::error!("Failed to retrieve podman info: {e}");
                        utils::show_error_toast(
                            obj.upcast_ref(),
                            &gettext("Failed to retrieve podman info"),
                            &e.to_string()
                        );
                    }
                }),
            );
        }

        self.imp().client.set(client);
        self.notify("client");
    }

    pub(crate) fn selected_image(&self) -> Option<model::ImageSearchResponse> {
        self.imp()
            .selection
            .get()
            .unwrap()
            .selected_item()
            .and_then(|item| item.downcast().ok())
    }

    pub(crate) fn tag(&self) -> glib::GString {
        let tag = self.imp().tag_entry_row.text();
        if tag.is_empty() {
            glib::GString::from(self.default_tag())
        } else {
            tag
        }
    }

    pub(crate) fn set_tag(&self, value: &str) {
        if self.tag().as_str() == value {
            return;
        }
        self.imp().tag_entry_row.set_text(value);
    }

    pub(crate) fn default_tag(&self) -> &'static str {
        "latest"
    }

    fn search(&self) {
        let imp = self.imp();

        if let Some(abort_handle) = imp.search_abort_handle.take() {
            abort_handle.abort();
        }

        let term = imp.search_entry_row.text();
        if term.is_empty() {
            imp.search_stack.set_visible_child_name("initial");
            return;
        }

        imp.search_stack.set_visible_child_name("searching");

        let (abort_handle, abort_registration) = future::AbortHandle::new_pair();
        imp.search_abort_handle.replace(Some(abort_handle));

        let opts = podman::opts::ImageSearchOpts::builder()
            .term(term.as_str())
            .build();
        utils::do_async(
            {
                let podman = self.client().unwrap().podman().clone();
                async move {
                    future::Abortable::new(podman.images().search(&opts), abort_registration).await
                }
            },
            clone!(@weak self as obj => move |result| if let Ok(responses) = result {
                match responses {
                    Ok(responses) => {
                        let imp = obj.imp();

                        imp.search_results.remove_all();

                        if responses.is_empty() {
                            imp.search_stack.set_visible_child_name("nothing");
                            imp.no_results_status_page.set_title(&gettext!("No Results For {}", term));
                        } else {
                            responses.into_iter().for_each(|response| {
                                obj.imp().search_results.append(&model::ImageSearchResponse::from(response));
                            });
                            imp.search_stack.set_visible_child_name("results");
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
}
