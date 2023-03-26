use std::cell::RefCell;

use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
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
        pub(super) client: glib::WeakRef<model::Client>,
        pub(super) term: RefCell<String>,
        pub(super) filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        pub(super) images_model: RefCell<Option<gio::ListModel>>,
        pub(super) containers_model: RefCell<Option<gio::ListModel>>,
        pub(super) pods_model: RefCell<Option<gio::ListModel>>,
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
        #[template_child]
        pub(super) pods_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) pods_list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchPanel {
        const NAME: &'static str = "PdsSearchPanel";
        type Type = super::SearchPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchPanel {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Client>("client")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecString::builder("term")
                        .explicit_notify()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "client" => obj.set_client(value.get().unwrap()),
                "term" => obj.set_term(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "client" => obj.client().to_value(),
                "term" => obj.term().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    let term = obj.term().to_uppercase();

                    if term.is_empty() {
                        false
                    } else if let Some(image) = item.downcast_ref::<model::Image>() {
                        image.id().contains(&term)
                        || image.repo_tags().contains(&term)
                    } else if let Some(container) = item.downcast_ref::<model::Container>() {
                        container
                            .name().to_uppercase().contains(&term)
                            || container
                                .id().contains(&term)
                            || container
                                .image_name()
                                .map(|image_name| image_name.to_uppercase().contains(&term))
                                .unwrap_or(false)
                            || container.image_id().contains(&term)
                    } else if let Some(pod) = item.downcast_ref::<model::Pod>() {
                        pod.name().to_uppercase().contains(&term)
                    } else {
                        unreachable!();
                    }
                }));

            let sorter = gtk::CustomSorter::new(|obj1, obj2| {
                if let Some(image1) = obj1.downcast_ref::<model::Image>() {
                    let image2 = obj2.downcast_ref::<model::Image>().unwrap();

                    if image1.repo_tags().n_items() == 0 {
                        if image2.repo_tags().n_items() == 0 {
                            image1.id().cmp(&image2.id()).into()
                        } else {
                            gtk::Ordering::Larger
                        }
                    } else if image2.repo_tags().n_items() == 0 {
                        gtk::Ordering::Smaller
                    } else {
                        image1.repo_tags().cmp(&image2.repo_tags()).into()
                    }
                } else if let Some(container1) = obj1.downcast_ref::<model::Container>() {
                    let container2 = obj2.downcast_ref::<model::Container>().unwrap();
                    container1.name().cmp(&container2.name()).into()
                } else if let Some(pod1) = obj1.downcast_ref::<model::Pod>() {
                    let pod2 = obj2.downcast_ref::<model::Pod>().unwrap();
                    pod1.name().cmp(&pod2.name()).into()
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

        fn dispose(&self) {
            self.main_stack.unparent();
        }
    }

    impl WidgetImpl for SearchPanel {}
}

glib::wrapper! {
    pub(crate) struct SearchPanel(ObjectSubclass<imp::SearchPanel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
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
            self.setup_model(
                client.image_list().upcast(),
                imp.images_list_box.get(),
                |item| view::ImageRow::from(item.downcast_ref().unwrap()).upcast(),
                &imp.images_model,
            );

            self.setup_model(
                client.container_list().upcast(),
                imp.containers_list_box.get(),
                |item| view::ContainerRow::from(item.downcast_ref().unwrap()).upcast(),
                &imp.containers_model,
            );

            self.setup_model(
                client.pod_list().upcast(),
                imp.pods_list_box.get(),
                |item| view::PodRow::from(item.downcast_ref().unwrap()).upcast(),
                &imp.pods_model,
            );

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

    fn setup_model<P: Fn(&glib::Object) -> gtk::Widget + 'static>(
        &self,
        model: gio::ListModel,
        list_box: gtk::ListBox,
        create_widget_func: P,
        this_model: &RefCell<Option<gio::ListModel>>,
    ) {
        let imp = self.imp();

        let model = gtk::SliceListModel::new(
            Some(gtk::SortListModel::new(
                Some(gtk::FilterListModel::new(
                    Some(model),
                    imp.filter.get().cloned(),
                )),
                imp.sorter.get().cloned(),
            )),
            0,
            8,
        );
        model.connect_items_changed(
            clone!(@weak self as obj => move |_, _, _, _| obj.update_view()),
        );
        list_box.bind_model(Some(&model), move |item| {
            create_widget_func(item.downcast_ref().unwrap())
        });
        this_model.replace(Some(model.upcast()));
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

        imp.pods_group.set_visible(
            imp.pods_model
                .borrow()
                .as_ref()
                .map(|model| model.n_items() > 0)
                .unwrap_or(false),
        );

        imp.main_stack.set_visible_child_name(
            if imp.images_group.is_visible()
                || imp.containers_group.is_visible()
                || imp.pods_group.is_visible()
            {
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
