use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::BinExt;
use adw::traits::ComboRowExt;
use futures::future;
use gettextrs::gettext;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::api;
use crate::model;
use crate::utils;
use crate::view;
use crate::window::Window;
use crate::PODMAN;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image-pull-page.ui")]
    pub(crate) struct ImagePullPage {
        pub(super) client: WeakRef<model::Client>,
        pub(super) search_results: gio::ListStore,
        pub(super) selection: OnceCell<gtk::SingleSelection>,
        pub(super) search_abort_handle: RefCell<Option<future::AbortHandle>>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
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
        #[template_child]
        pub(super) image_page_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePullPage {
        const NAME: &'static str = "ImagePullPage";
        type Type = super::ImagePullPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("navigation.go-first", None, move |widget, _, _| {
                widget.navigate_to_first();
            });
            klass.install_action("navigation.back", None, move |widget, _, _| {
                widget.navigate_back();
            });

            klass.install_action("image.pull", None, |widget, _, _| {
                widget.pull();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagePullPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "client",
                    "Client",
                    "The client of this ImagePullPage",
                    model::Client::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "client" => self.client.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "client" => obj.client().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

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
                        log::error!("Failed to retrieve registries: {e}");
                        utils::show_error_toast(
                            &obj,
                            &gettext("Failed to retrieve registries"),
                            &e.to_string()
                        );
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

        fn dispose(&self, _obj: &Self::Type) {
            self.stack.unparent();
        }
    }

    impl WidgetImpl for ImagePullPage {
        fn realize(&self, widget: &Self::Type) {
            self.parent_realize(widget);

            widget.action_set_enabled(
                "navigation.go-first",
                widget.previous_leaflet_overlay() != widget.root_leaflet_overlay(),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImagePullPage(ObjectSubclass<imp::ImagePullPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Client> for ImagePullPage {
    fn from(client: &model::Client) -> Self {
        glib::Object::new(&[("client", client)]).expect("Failed to create ImagePullPage")
    }
}

impl ImagePullPage {
    fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    fn pull(&self) {
        let imp = self.imp();

        if let Some(search_response) = imp
            .selection
            .get()
            .unwrap()
            .selected_item()
            .and_then(|i| i.downcast::<model::ImageSearchResponse>().ok())
        {
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

            imp.stack.set_visible_child_name("waiting");

            utils::do_async(
                async move { PODMAN.images().pull(&opts).await },
                clone!(@weak self as obj => move |result| {
                    let imp = obj.imp();

                    match result {
                        Ok(report) => match report.error {
                            Some(error) => obj.on_pull_error(&error),
                            None => {
                                let image_id = report.id.unwrap();
                                let client = imp.client.upgrade().unwrap();
                                match client.image_list().get_image(&image_id) {
                                    Some(image) => obj.switch_to_image(&image),
                                    None => {
                                        client.image_list().connect_image_added(
                                            clone!(@weak obj => move |_, image| {
                                                if image.id() == image_id.as_str() {
                                                    obj.switch_to_image(image);
                                                }
                                            }),
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => obj.on_pull_error(&e.to_string()),
                    }
                }),
            );
        }
    }

    fn on_pull_error(&self, msg: &str) {
        self.imp().stack.set_visible_child_name("pull-settings");
        log::error!("Failed to pull image: {}", msg);
        utils::show_error_toast(self, &gettext("Failed to pull image"), msg);
    }

    fn switch_to_image(&self, image: &model::Image) {
        let imp = self.imp();
        imp.image_page_bin
            .set_child(Some(&view::ImageDetailsPage::from(image)));
        imp.stack.set_visible_child(&*imp.image_page_bin);
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
                            &obj,
                            &gettext("Failed to search for images"),
                            &e.to_string());
                    }
                }
            }),
        );
    }

    fn navigate_to_first(&self) {
        self.root_leaflet_overlay().hide_details();
    }

    fn navigate_back(&self) {
        self.previous_leaflet_overlay().hide_details();
    }

    fn previous_leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::find_leaflet_overlay(self)
    }

    fn root_leaflet_overlay(&self) -> view::LeafletOverlay {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .leaflet_overlay()
    }
}
