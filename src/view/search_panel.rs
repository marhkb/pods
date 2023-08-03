use std::cell::OnceCell;
use std::cell::RefCell;

use glib::clone;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::model::AbstractContainerListExt;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::SearchPanel)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/search_panel.ui")]
    pub(crate) struct SearchPanel {
        pub(super) filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        pub(super) containers_model: RefCell<Option<gio::ListModel>>,
        pub(super) pods_model: RefCell<Option<gio::ListModel>>,
        pub(super) images_model: RefCell<Option<gio::ListModel>>,
        pub(super) volumes_model: RefCell<Option<gio::ListModel>>,
        #[property(get, set = Self::set_client, explicit_notify, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) containers_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) containers_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) pods_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) pods_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) images_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) images_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) volumes_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) volumes_list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchPanel {
        const NAME: &'static str = "PdsSearchPanel";
        type Type = super::SearchPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchPanel {
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

            let filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    let term = obj.imp().search_entry.text().to_lowercase();

                    if term.is_empty() {
                        false
                    } else if let Some(container) = item.downcast_ref::<model::Container>() {
                        container
                            .name().to_lowercase().contains(&term)
                            || container.id().contains(&term)
                            || container
                                .image_name()
                                .map(|image_name| image_name.to_lowercase().contains(&term))
                                .unwrap_or(false)
                            || container.image_id().contains(&term)
                    } else if let Some(pod) = item.downcast_ref::<model::Pod>() {
                        pod.name().to_lowercase().contains(&term)
                    } else if let Some(image) = item.downcast_ref::<model::Image>() {
                        image.id().contains(&term) || image.repo_tags().contains(&term)
                    } else if let Some(volume) = item.downcast_ref::<model::Volume>() {
                        volume.inner().name.to_lowercase().contains(&term)
                    } else {
                        unreachable!();
                    }
                }));

            let sorter = gtk::CustomSorter::new(|obj1, obj2| {
                if let Some(container1) = obj1.downcast_ref::<model::Container>() {
                    let container2 = obj2.downcast_ref::<model::Container>().unwrap();
                    container1.name().cmp(&container2.name()).into()
                } else if let Some(pod1) = obj1.downcast_ref::<model::Pod>() {
                    let pod2 = obj2.downcast_ref::<model::Pod>().unwrap();
                    pod1.name().cmp(&pod2.name()).into()
                } else if let Some(image1) = obj1.downcast_ref::<model::Image>() {
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
                } else if let Some(volume1) = obj1.downcast_ref::<model::Volume>() {
                    let volume2 = obj2.downcast_ref::<model::Volume>().unwrap();
                    volume1.inner().name.cmp(&volume2.inner().name).into()
                } else {
                    unreachable!();
                }
            });

            self.filter.set(filter.upcast()).unwrap();
            self.sorter.set(sorter.upcast()).unwrap();
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for SearchPanel {
        fn map(&self) {
            self.parent_map();
            self.search_entry
                .set_key_capture_widget(Some(&utils::root(self.obj().upcast_ref())));
            self.search_entry.grab_focus();
            self.search_entry.select_region(0, -1);
        }

        fn unmap(&self) {
            self.parent_unmap();
            self.search_entry.set_key_capture_widget(gtk::Widget::NONE);
        }
    }

    #[gtk::template_callbacks]
    impl SearchPanel {
        #[template_callback]
        fn on_search_changed(&self) {
            let obj = &*self.obj();

            obj.update_filter();
            if self.search_entry.text().is_empty() {
                obj.update_view();
            }
        }

        pub(super) fn set_client(&self, value: Option<&model::Client>) {
            let obj = &*self.obj();
            if obj.client().as_ref() == value {
                return;
            }

            if let Some(client) = value {
                obj.setup_model(
                    client.container_list().upcast(),
                    self.containers_list_box.get(),
                    |item| view::ContainerRow::from(item.downcast_ref().unwrap()).upcast(),
                    &self.containers_model,
                );

                obj.setup_model(
                    client.pod_list().upcast(),
                    self.pods_list_box.get(),
                    |item| view::PodRow::from(item.downcast_ref().unwrap()).upcast(),
                    &self.pods_model,
                );

                obj.setup_model(
                    client.image_list().upcast(),
                    self.images_list_box.get(),
                    |item| view::ImageRow::from(item.downcast_ref().unwrap()).upcast(),
                    &self.images_model,
                );

                obj.setup_model(
                    client.volume_list().upcast(),
                    self.volumes_list_box.get(),
                    |item| view::VolumeRow::from(item.downcast_ref().unwrap()).upcast(),
                    &self.volumes_model,
                );

                client.container_list().connect_container_name_changed(
                    clone!(@weak obj => move |_, _| {
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

            self.client.set(value);
            obj.notify_client();
        }
    }
}

glib::wrapper! {
    pub(crate) struct SearchPanel(ObjectSubclass<imp::SearchPanel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SearchPanel {
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
            6,
        );
        model.connect_items_changed(
            clone!(@weak self as obj => move |_, _, _, _| obj.update_view()),
        );
        list_box.bind_model(Some(&model), move |item| {
            create_widget_func(item.downcast_ref().unwrap())
        });
        this_model.replace(Some(model.upcast()));
    }

    fn update_view(&self) {
        let imp = self.imp();

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

        imp.images_group.set_visible(
            imp.images_model
                .borrow()
                .as_ref()
                .map(|model| model.n_items() > 0)
                .unwrap_or(false),
        );

        imp.volumes_group.set_visible(
            imp.volumes_model
                .borrow()
                .as_ref()
                .map(|model| model.n_items() > 0)
                .unwrap_or(false),
        );

        imp.main_stack.set_visible_child_name(
            if imp.containers_group.is_visible()
                || imp.pods_group.is_visible()
                || imp.images_group.is_visible()
                || imp.volumes_group.is_visible()
            {
                "results"
            } else if imp.search_entry.text().is_empty() {
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
