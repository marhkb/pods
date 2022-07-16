use std::cell::RefCell;

use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::model;
use crate::model::AbstractContainerListExt;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/search-panel.ui")]
    pub(crate) struct SearchPanel {
        pub(super) client: WeakRef<model::Client>,
        pub(super) term: RefCell<String>,
        pub(super) filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        pub(super) images_model: RefCell<Option<gio::ListModel>>,
        pub(super) containers_model: RefCell<Option<gio::ListModel>>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) images_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) images_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) containers_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) containers_list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchPanel {
        const NAME: &'static str = "SearchPanel";
        type Type = super::SearchPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchPanel {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "client",
                        "Client",
                        "The client",
                        model::Client::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecString::new(
                        "term",
                        "Term",
                        "The term to search for",
                        None,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "client" => obj.set_client(value.get().unwrap()),
                "term" => obj.set_term(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "client" => obj.client().to_value(),
                "term" => obj.term().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    let term = obj.term();

                    if term.is_empty() {
                        false
                    } else if let Some(image) = item.downcast_ref::<model::Image>() {
                        image.id().contains(&term)
                        || image.repo_tags().iter().any(|s| s.contains(&term))
                    } else if let Some(container) = item.downcast_ref::<model::Container>() {
                        container
                            .name()
                            .map(|name| name.contains(&term))
                            .unwrap_or(false)
                            || container
                                .id()
                                .map(|id| id.contains(&term))
                                .unwrap_or(false)
                            || container
                                .image_name()
                                .map(|image_name| image_name.contains(&term))
                                .unwrap_or(false)
                            || container
                                .image_id()
                                .map(|image_id| image_id.contains(&term))
                                .unwrap_or(false)
                    } else {
                        unreachable!();
                    }
                }));

            let sorter = gtk::CustomSorter::new(|obj1, obj2| {
                if let Some(image1) = obj1.downcast_ref::<model::Image>() {
                    let image2 = obj2.downcast_ref::<model::Image>().unwrap();

                    if image1.repo_tags().is_empty() {
                        if image2.repo_tags().is_empty() {
                            image1.id().cmp(image2.id()).into()
                        } else {
                            gtk::Ordering::Larger
                        }
                    } else if image2.repo_tags().is_empty() {
                        gtk::Ordering::Smaller
                    } else {
                        image1.repo_tags().cmp(image2.repo_tags()).into()
                    }
                } else if let Some(container1) = obj1.downcast_ref::<model::Container>() {
                    let container2 = obj2.downcast_ref::<model::Container>().unwrap();
                    container1.name().cmp(&container2.name()).into()
                } else {
                    unreachable!();
                }
            });

            self.filter.set(filter.upcast()).unwrap();
            self.sorter.set(sorter.upcast()).unwrap();

            obj.connect_notify_local(Some("term"), |obj, _| {
                obj.update_filter();
                obj.update_view();
            });
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.main_stack.unparent();
        }
    }

    impl WidgetImpl for SearchPanel {}
}

glib::wrapper! {
    pub(crate) struct SearchPanel(ObjectSubclass<imp::SearchPanel>)
        @extends gtk::Widget;
}

impl SearchPanel {
    pub(crate) fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    pub(crate) fn set_client(&self, value: Option<&model::Client>) {
        if self.client().as_ref() == value {
            return;
        }

        let imp = self.imp();

        if let Some(client) = value {
            let images_model = gtk::SliceListModel::new(
                Some(&gtk::SortListModel::new(
                    Some(&gtk::FilterListModel::new(
                        Some(client.image_list()),
                        imp.filter.get(),
                    )),
                    imp.sorter.get(),
                )),
                0,
                8,
            );
            images_model.connect_items_changed(
                clone!(@weak self as obj => move |_, _, _, _| obj.update_view()),
            );
            imp.images_list_box.bind_model(Some(&images_model), |item| {
                view::ImageRow::from(item.downcast_ref().unwrap()).upcast()
            });
            imp.images_model.replace(Some(images_model.upcast()));

            let containers_model = gtk::SliceListModel::new(
                Some(&gtk::SortListModel::new(
                    Some(&gtk::FilterListModel::new(
                        Some(client.container_list()),
                        imp.filter.get(),
                    )),
                    imp.sorter.get(),
                )),
                0,
                8,
            );
            containers_model.connect_items_changed(
                clone!(@weak self as obj => move |_, _, _, _| obj.update_view()),
            );
            imp.containers_list_box
                .bind_model(Some(&containers_model), |item| {
                    view::ContainerRow::from(item.downcast_ref().unwrap()).upcast()
                });
            imp.containers_model
                .replace(Some(containers_model.upcast()));

            client.container_list().connect_container_name_changed(
                clone!(@weak self as obj => move |_, _| {
                    glib::timeout_add_seconds_local_once(
                        1,
                        clone!(@weak obj => move || {
                            obj.update_filter();
                            obj.update_sorter();
                        }),
                    );
                }),
            );
        }

        imp.client.set(value);
        self.notify("client");
    }

    pub(crate) fn term(&self) -> String {
        self.imp().term.borrow().clone()
    }

    pub(crate) fn set_term(&self, value: String) {
        if self.term() == value {
            return;
        }
        self.imp().term.replace(value);
        self.notify("term");
    }

    fn update_view(&self) {
        let imp = self.imp();

        imp.images_group.set_visible(
            imp.images_model
                .borrow()
                .as_ref()
                .map(|model| model.n_items() > 0)
                .unwrap_or(false),
        );

        imp.containers_group.set_visible(
            imp.containers_model
                .borrow()
                .as_ref()
                .map(|model| model.n_items() > 0)
                .unwrap_or(false),
        );

        imp.main_stack.set_visible_child_name(
            if imp.images_group.is_visible() || imp.containers_group.is_visible() {
                "results"
            } else if self.term().is_empty() {
                "search"
            } else {
                "no-results"
            },
        );
    }

    fn update_filter(&self) {
        self.imp()
            .filter
            .get()
            .unwrap()
            .changed(gtk::FilterChange::Different);
    }

    fn update_sorter(&self) {
        self.imp()
            .sorter
            .get()
            .unwrap()
            .changed(gtk::SorterChange::Different);
    }
}
