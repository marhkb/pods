use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::ComboRowExt;
use futures::future;
use gettextrs::gettext;
use glib::clone;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::OnceCell as SyncOnceCell;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageSearchWidget)]
    #[template(file = "image_search_widget.ui")]
    pub(crate) struct ImageSearchWidget {
        pub(super) search_results: gio::ListStore,
        pub(super) search_abort_handle: RefCell<Option<future::AbortHandle>>,
        #[property(get, set = Self::set_client, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
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
        pub(super) list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) selection: TemplateChild<gtk::SingleSelection>,
        #[template_child]
        pub(super) tag_entry_row: TemplateChild<adw::EntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageSearchWidget {
        const NAME: &'static str = "PdsImageSearchWidget";
        type Type = super::ImageSearchWidget;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageSearchWidget {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: SyncOnceCell<Vec<glib::ParamSpec>> = SyncOnceCell::new();
            PROPERTIES.get_or_init(|| {
                Self::derived_properties()
                    .iter()
                    .cloned()
                    .chain(vec![
                        glib::ParamSpecObject::builder::<model::ImageSearchResponse>(
                            "selected-image",
                        )
                        .read_only()
                        .build(),
                        glib::ParamSpecString::builder("tag")
                            .explicit_notify()
                            .build(),
                        glib::ParamSpecString::builder("default-tag")
                            .default_value(Some("latest"))
                            .read_only()
                            .build(),
                    ])
                    .collect::<Vec<_>>()
            })
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "tag" => self.obj().set_tag(value.get().unwrap()),
                _ => self.derived_set_property(id, value, pspec),
            }
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "selected-image" => self.obj().selected_image().to_value(),
                "tag" => self.obj().tag().to_value(),
                "default-tag" => self.obj().default_tag().to_value(),
                _ => self.derived_property(id, pspec),
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
                                .name(),
                        ) == item
                            .downcast_ref::<model::ImageSearchResponse>()
                            .unwrap()
                            .index()
                }));

            self.registries_combo_row.connect_selected_item_notify(
                clone!(@weak filter => move |_| filter.changed(gtk::FilterChange::Different)),
            );

            self.selection.set_model(Some(&gtk::FilterListModel::new(
                Some(self.search_results.clone()),
                Some(filter),
            )));

            let list_factory = gtk::SignalListItemFactory::new();
            list_factory.connect_bind(clone!(@weak obj => move |_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();

                if let Some(item) = list_item.item() {
                    let response = item.downcast::<model::ImageSearchResponse>().unwrap();
                    let row = view::ImageSearchResponseRow::from(&response);

                    list_item.set_child(Some(&row));
                }
            }));
            list_factory.connect_unbind(|_, list_item| {
                list_item
                    .downcast_ref::<gtk::ListItem>()
                    .unwrap()
                    .set_child(gtk::Widget::NONE);
            });
            self.list_view.set_factory(Some(&list_factory));

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
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ImageSearchWidget {
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

    impl PreferencesGroupImpl for ImageSearchWidget {}

    #[gtk::template_callbacks]
    impl ImageSearchWidget {
        #[template_callback]
        fn on_image_selected(&self) {
            self.obj().notify("selected-image");
        }

        #[template_callback]
        fn on_image_activated(&self, _: u32) {
            self.obj()
                .activate_action(<Self as ObjectSubclass>::Type::action_select(), None)
                .unwrap();
        }

        pub(super) fn set_client(&self, client: Option<&model::Client>) {
            let obj = &*self.obj();
            if obj.client().as_ref() == client {
                return;
            }

            if let Some(client) = client {
                utils::do_async(
                    {
                        let podman = client.podman();
                        async move { podman.info().await }
                    },
                    clone!(@weak obj => move |result| match result {
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

            self.client.set(client);
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageSearchWidget(ObjectSubclass<imp::ImageSearchWidget>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ImageSearchWidget {
    pub(crate) fn action_select() -> &'static str {
        "image-search-widget.select"
    }

    pub(crate) fn selected_image(&self) -> Option<model::ImageSearchResponse> {
        self.imp()
            .selection
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

        imp.search_results.remove_all();
        self.notify("selected-image");

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
                let podman = self.client().unwrap().podman();
                async move {
                    future::Abortable::new(podman.images().search(&opts), abort_registration).await
                }
            },
            clone!(@weak self as obj => move |result| if let Ok(responses) = result {
                match responses {
                    Ok(responses) => {
                        let imp = obj.imp();

                        if responses.is_empty() {
                            imp.search_stack.set_visible_child_name("nothing");
                            imp.no_results_status_page.set_title(&gettext!("No Results For {}", term));
                        } else {
                            responses.into_iter().for_each(|response| {
                                imp.search_results.append(&model::ImageSearchResponse::from(response));
                            });
                            imp.search_stack.set_visible_child_name("results");

                            obj.notify("selected-image");
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
