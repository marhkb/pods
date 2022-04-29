use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::ComboRowExt;
use futures::future;
use gettextrs::gettext;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::unsync::OnceCell;

use crate::api;
use crate::model;
use crate::utils;
use crate::PODMAN;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image-pull-dialog.ui")]
    pub(crate) struct ImagePullDialog {
        pub(super) search_results: gio::ListStore,
        pub(super) selection: OnceCell<gtk::SingleSelection>,
        pub(super) pull_abort_handle: RefCell<Option<future::AbortHandle>>,
        pub(super) search_abort_handle: RefCell<Option<future::AbortHandle>>,
        #[template_child]
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) pull_controll_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) pull_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) registries_combo_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) search_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) no_results_status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) search_result_list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) tag_entry: TemplateChild<gtk::Entry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePullDialog {
        const NAME: &'static str = "ImagePullDialog";
        type Type = super::ImagePullDialog;
        type ParentType = gtk::Dialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("image.pull", None, |widget, _, _| {
                widget.pull();
            });
            klass.install_action("image.cancel-pull", None, |widget, _, _| {
                widget.cancel_pull();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagePullDialog {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.set_default_widget(Some(&*self.pull_button));

            self.search_entry
                .connect_changed(clone!(@weak obj => move |_| obj.search()));

            self.registries_combo_row
                .set_expression(Some(&model::Registry::this_expression("name")));

            utils::do_async(
                PODMAN.info(),
                clone!(@weak obj => move |result| match result {
                    Ok(info) => {

                        let model = gio::ListStore::new(model::Registry::static_type());
                        model.append(&model::Registry::from(gettext("All registries").as_str()));
                        info.registries
                            .unwrap()
                            .get("search")
                            .unwrap()
                            .as_array()
                            .unwrap()
                            .iter()
                            .for_each(|name| {
                                model.append(&model::Registry::from(name.as_str().unwrap()))
                            });

                        obj.imp().registries_combo_row.set_model(Some(&model));
                    }
                    Err(e) => {
                        log::error!("Failed to retrieve registries: {}", e);
                        obj.show_toast(&gettext!("Failed to retrieve registries: {}", e));
                    }
                }),
            );

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

            let selection = gtk::SingleSelection::new(Some(&gtk::FilterListModel::new(
                Some(&self.search_results),
                Some(&filter),
            )));
            obj.action_set_enabled("image.pull", false);
            selection.connect_selected_item_notify(clone!(@weak obj => move |selection| {
                obj.action_set_enabled("image.pull", selection.selected_item().is_some());
            }));

            self.search_result_list_view.set_model(Some(&selection));

            self.selection.set(selection).unwrap();
        }
    }

    impl WidgetImpl for ImagePullDialog {
        fn show(&self, widget: &Self::Type) {
            self.parent_show(widget);
            self.search_entry.grab_focus();
        }
    }

    impl WindowImpl for ImagePullDialog {}
    impl DialogImpl for ImagePullDialog {}
}

glib::wrapper! {
    pub(crate) struct ImagePullDialog(ObjectSubclass<imp::ImagePullDialog>)
        @extends gtk::Widget, gtk::Window, gtk::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl From<&Option<gtk::Window>> for ImagePullDialog {
    fn from(parent_window: &Option<gtk::Window>) -> Self {
        glib::Object::new(&[("transient-for", parent_window), ("use-header-bar", &1)])
            .expect("Failed to create ImagePullDialog")
    }
}

impl ImagePullDialog {
    fn pull(&self) {
        let imp = self.imp();

        if let Some(search_response) = imp
            .selection
            .get()
            .unwrap()
            .selected_item()
            .and_then(|i| i.downcast::<model::ImageSearchResponse>().ok())
        {
            imp.pull_controll_stack.set_visible_child_name("cancel");
            self.action_set_enabled("image.pull", false);

            let (abort_handle, abort_registration) = future::AbortHandle::new_pair();
            if let Some(old_abort_handle) = imp.pull_abort_handle.replace(Some(abort_handle)) {
                old_abort_handle.abort();
            }

            let tag = imp.tag_entry.text();
            let opts = api::PullOpts::builder()
                .reference(format!(
                    "{}:{}",
                    search_response.name().unwrap(),
                    if tag.is_empty() {
                        "latest"
                    } else {
                        tag.as_str()
                    }
                ))
                .quiet(true)
                .build();

            utils::do_async(
                async move {
                    future::Abortable::new(PODMAN.images().pull(&opts), abort_registration).await
                },
                clone!(@weak self as obj => move |result| {
                obj.imp().pull_controll_stack.set_visible_child_name("pull");
                obj.action_set_enabled("image.pull", true);

                if let Ok(result) = result {
                    match result {
                        Ok(_) => obj.response(gtk::ResponseType::Close),
                        Err(e) => {
                            log::error!("Failed to pull image: {}", e);
                            obj.show_toast(&gettext!("Failed to pull image: {}", e));
                        }
                    }
                }}),
            );
        }
    }

    fn cancel_pull(&self) {
        if let Some(abort_handle) = self.imp().pull_abort_handle.take() {
            abort_handle.abort();
        }
    }

    fn search(&self) {
        let imp = self.imp();

        if let Some(abort_handle) = imp.search_abort_handle.take() {
            abort_handle.abort();
        }

        let term = imp.search_entry.text();
        if term.is_empty() {
            imp.search_stack.set_visible_child_name("initial");
            return;
        }

        imp.search_stack.set_visible_child_name("searching");

        let (abort_handle, abort_registration) = future::AbortHandle::new_pair();
        imp.search_abort_handle.replace(Some(abort_handle));

        let opts = api::ImageSearchOpts::builder().term(term.as_str()).build();
        utils::do_async(
            async move {
                future::Abortable::new(PODMAN.images().search(&opts), abort_registration).await
            },
            clone!(@weak self as obj => move |result| if let Ok(responses) = result {
                match responses {
                    Ok(responses) => {
                        let imp = obj.imp();

                        imp.search_results.remove_all();

                        if responses.is_empty() {
                            imp.search_stack.set_visible_child_name("nothing");
                            imp.no_results_status_page.set_title(&gettext!("No results for {}", term));
                        } else {
                            responses.into_iter().for_each(|response| {
                                obj.imp().search_results.append(&model::ImageSearchResponse::from(response));
                            });
                            imp.search_stack.set_visible_child_name("results");
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to search for images: {}", e);
                        obj.show_toast(&gettext!("Failed to search for images: {}", e));
                    }
                }
            }),
        );
    }

    fn show_toast(&self, title: &str) {
        self.imp().toast_overlay.add_toast(
            &adw::Toast::builder()
                .title(title)
                .timeout(3)
                .priority(adw::ToastPriority::High)
                .build(),
        );
    }
}
