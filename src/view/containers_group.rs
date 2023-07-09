use adw::subclass::prelude::PreferencesGroupImpl;
use glib::clone;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::unsync::OnceCell as UnsyncOnceCell;

use crate::model;
use crate::model::AbstractContainerListExt;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainersGroup)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/containers_group.ui")]
    pub(crate) struct ContainersGroup {
        pub(super) sorter: UnsyncOnceCell<gtk::Sorter>,
        #[property(get, set = Self::set_container_list, nullable)]
        pub(super) container_list: glib::WeakRef<model::AbstractContainerList>,
        #[template_child]
        pub(super) create_container_row: TemplateChild<gtk::ListBoxRow>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainersGroup {
        const NAME: &'static str = "PdsContainersGroup";
        type Type = super::ContainersGroup;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainersGroup {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            let sorter = gtk::CustomSorter::new(|item1, item2| {
                item1
                    .downcast_ref::<model::Container>()
                    .unwrap()
                    .name()
                    .to_lowercase()
                    .cmp(
                        &item2
                            .downcast_ref::<model::Container>()
                            .unwrap()
                            .name()
                            .to_lowercase(),
                    )
                    .into()
            });
            self.sorter.set(sorter.upcast()).unwrap();
        }
    }

    impl WidgetImpl for ContainersGroup {}
    impl PreferencesGroupImpl for ContainersGroup {}

    impl ContainersGroup {
        pub(super) fn set_container_list(&self, value: Option<&model::AbstractContainerList>) {
            let obj = &*self.obj();
            if obj.container_list().as_ref() == value {
                return;
            }

            if let Some(list) = value {
                list.connect_container_name_changed(clone!(@weak obj => move |_, _| {
                    glib::timeout_add_seconds_local_once(
                        1,
                        clone!(@weak obj => move || obj.update_sorter()),
                    );
                }));

                let model =
                    gtk::SortListModel::new(Some(list.to_owned()), self.sorter.get().cloned());

                self.list_box.bind_model(Some(&model), |item| {
                    view::ContainerRow::from(item.downcast_ref().unwrap()).upcast()
                });
                self.list_box.append(&*self.create_container_row);
            }

            self.container_list.set(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainersGroup(ObjectSubclass<imp::ContainersGroup>)
        @extends gtk::Widget, adw::PreferencesGroup,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ContainersGroup {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl ContainersGroup {
    fn update_sorter(&self) {
        self.imp()
            .sorter
            .get()
            .unwrap()
            .changed(gtk::SorterChange::Different);
    }

    pub(crate) fn action_create_container() -> &'static str {
        "containers-group.create-container"
    }
}
