use std::cell::OnceCell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use futures::future;
use gettextrs::gettext;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::rt;
use crate::utils;
use crate::widget;

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageSuggestionEntryRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_suggestion_entry_row.ui")]
    pub(crate) struct ImageSuggestionEntryRow {
        pub(super) search_abort_handle: RefCell<Option<future::AbortHandle>>,

        #[property(get = Self::search_results)]
        pub(super) search_results: OnceCell<gio::ListStore>,
        #[property(get, set, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageSuggestionEntryRow {
        const NAME: &'static str = "PdsImageSuggestionEntryRow";
        type Type = super::ImageSuggestionEntryRow;
        type ParentType = widget::SuggestionEntryRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageSuggestionEntryRow {
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

            obj.upcast_ref::<widget::SuggestionEntryRow>()
                .set_model(Some(
                    gtk::SortListModel::new(
                        Some(self.search_results().to_owned()),
                        Some(
                            gtk::NumericSorter::builder()
                                .sort_order(gtk::SortType::Descending)
                                .expression(model::ImageSearchResponse::this_expression("stars"))
                                .build(),
                        ),
                    )
                    .upcast_ref(),
                ));
        }

        fn dispose(&self) {
            if let Some(abort_handle) = self.search_abort_handle.take() {
                abort_handle.abort();
            }
        }
    }

    impl WidgetImpl for ImageSuggestionEntryRow {}

    #[gtk::template_callbacks]
    impl ImageSuggestionEntryRow {
        #[template_callback]
        async fn on_changed(&self) {
            let obj = &*self.obj();

            if let Some(abort_handle) = self.search_abort_handle.take() {
                abort_handle.abort();
            }

            let Some(client) = obj.client() else {
                return;
            };

            self.search_results().remove_all();

            let term = obj.text();
            if term.is_empty() {
                obj.popdown();
                return;
            }

            obj.set_visible_stack_page(widget::SuggestionEntryVisibleStackPage::Searching);

            let (abort_handle, abort_registration) = future::AbortHandle::new_pair();
            self.search_abort_handle.replace(Some(abort_handle));

            let result = rt::Promise::new({
                let images = client.engine().images();
                let term = term.clone().into();
                async move { future::Abortable::new(images.search(term), abort_registration).await }
            })
            .exec()
            .await;

            let Ok(responses) = result else {
                return;
            };

            match responses {
                Ok(responses) => {
                    if responses.is_empty() {
                        obj.popdown();
                    } else {
                        responses.into_iter().for_each(|response| {
                            self.search_results()
                                .append(&model::ImageSearchResponse::from(response));
                        });

                        obj.set_visible_stack_page(
                            widget::SuggestionEntryVisibleStackPage::Results,
                        );
                    }
                }
                Err(e) => {
                    log::error!("Failed to search for images: {}", e);
                    utils::show_error_toast(
                        obj,
                        &gettext("Failed to search for images"),
                        &e.to_string(),
                    );
                }
            }
        }

        fn search_results(&self) -> gio::ListStore {
            self.search_results
                .get_or_init(gio::ListStore::new::<model::ImageSearchResponse>)
                .to_owned()
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageSuggestionEntryRow(ObjectSubclass<imp::ImageSuggestionEntryRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::EntryRow, widget::SuggestionEntryRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget, gtk::Editable;
}

impl From<&model::Client> for ImageSuggestionEntryRow {
    fn from(value: &model::Client) -> Self {
        glib::Object::builder().property("client", value).build()
    }
}

impl ImageSuggestionEntryRow {
    pub(crate) fn popdown(&self) {
        self.upcast_ref::<widget::SuggestionEntryRow>().popdown();
    }

    fn set_visible_stack_page(&self, value: widget::SuggestionEntryVisibleStackPage) {
        self.upcast_ref::<widget::SuggestionEntryRow>()
            .set_visible_stack_page(value);
    }
}
