use std::cell::RefCell;

use adw::subclass::prelude::PreferencesGroupImpl;
use gettextrs::gettext;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::model;
use crate::model::AbstractContainerListExt;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/containers-group.ui")]
    pub(crate) struct ContainersGroup {
        pub(super) settings: utils::PodsSettings,
        pub(super) container_list: WeakRef<model::AbstractContainerList>,
        pub(super) no_containers_label: RefCell<Option<String>>,
        pub(super) show_running_settings_key: RefCell<String>,
        pub(super) properties_filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        #[template_child]
        pub(super) show_only_running_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainersGroup {
        const NAME: &'static str = "ContainersGroup";
        type Type = super::ContainersGroup;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainersGroup {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::new(
                        "no-containers-label",
                        "No Containers Label",
                        "The description label if no containers are present",
                        None,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecString::new(
                        "show-running-settings-key",
                        "Show Running Settings Key",
                        "The settings key for showing running key",
                        None,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecObject::new(
                        "container-list",
                        "Container List",
                        "The list of containers",
                        model::AbstractContainerList::static_type(),
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
                "no-containers-label" => obj.set_no_containers_label(value.get().unwrap()),
                "show-running-settings-key" => {
                    obj.set_show_running_settings_key(value.get().unwrap_or_default());
                }
                "container-list" => obj.set_container_list(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "no-containers-label" => obj.no_containers_label().to_value(),
                "show-running-settings-key" => obj.show_running_settings_key().to_value(),
                "container-list" => obj.container_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let container_list_expr = Self::Type::this_expression("container-list");
            gtk::ClosureExpression::new::<Option<String>, _, _>(
                &[
                    container_list_expr.chain_property::<model::AbstractContainerList>("len"),
                    container_list_expr.chain_property::<model::AbstractContainerList>("running"),
                ],
                closure!(|obj: Self::Type, len: u32, running: u32| {
                    if len > 0 {
                        Some(gettext!(
                            // Translators: There's a wide space (U+2002) between ", {}".
                            "{} Containers total, {} running",
                            len,
                            running
                        ))
                    } else {
                        obj.no_containers_label()
                    }
                }),
            )
            .bind(obj, "description", Some(obj));

            let properties_filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    !obj.imp().show_only_running_switch.is_active() ||
                        item.downcast_ref::<model::Container>().unwrap().status()
                            == model::ContainerStatus::Running
                }));

            let sorter = gtk::CustomSorter::new(|obj1, obj2| {
                let container1 = obj1.downcast_ref::<model::Container>().unwrap();
                let container2 = obj2.downcast_ref::<model::Container>().unwrap();

                container1.name().cmp(&container2.name()).into()
            });

            self.properties_filter
                .set(properties_filter.upcast())
                .unwrap();
            self.sorter.set(sorter.upcast()).unwrap();

            self.show_only_running_switch.connect_active_notify(
                clone!(@weak obj => move |_| obj.update_properties_filter()),
            );
        }
    }

    impl WidgetImpl for ContainersGroup {}
    impl PreferencesGroupImpl for ContainersGroup {}
}

glib::wrapper! {
    pub(crate) struct ContainersGroup(ObjectSubclass<imp::ContainersGroup>)
        @extends gtk::Widget, adw::PreferencesGroup;
}

impl Default for ContainersGroup {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create ContainersGroup")
    }
}

impl ContainersGroup {
    pub(crate) fn no_containers_label(&self) -> Option<String> {
        self.imp().no_containers_label.borrow().to_owned()
    }

    pub(crate) fn set_no_containers_label(&self, value: Option<String>) {
        if self.no_containers_label() == value {
            return;
        }
        self.imp().no_containers_label.replace(value);
        self.notify("no-containers-label");
    }

    pub(crate) fn show_running_settings_key(&self) -> String {
        self.imp().show_running_settings_key.borrow().to_owned()
    }

    pub(crate) fn set_show_running_settings_key(&self, value: String) {
        if self.show_running_settings_key() == value {
            return;
        }

        let imp = self.imp();

        imp.settings
            .bind(&value, &*imp.show_only_running_switch, "active")
            .build();

        imp.show_running_settings_key.replace(value);
        self.notify("show-running-settings-key");
    }

    pub(crate) fn container_list(&self) -> Option<model::AbstractContainerList> {
        self.imp().container_list.upgrade()
    }

    pub(crate) fn set_container_list(&self, value: Option<&model::AbstractContainerList>) {
        if self.container_list().as_ref() == value {
            return;
        }

        let imp = self.imp();

        if let Some(value) = value {
            // TODO: For multi-client: Figure out whether signal handlers need to be disconnected.
            value.connect_notify_local(
                Some("running"),
                clone!(@weak self as obj => move |_, _| obj.update_properties_filter()),
            );

            value.connect_container_name_changed(clone!(@weak self as obj => move |_, _| {
                glib::timeout_add_seconds_local_once(
                    1,
                    clone!(@weak obj => move || obj.update_sorter()),
                );
            }));

            let model = gtk::SortListModel::new(
                Some(&gtk::FilterListModel::new(
                    Some(value),
                    imp.properties_filter.get(),
                )),
                imp.sorter.get(),
            );

            self.set_list_box_visibility(model.upcast_ref());
            model.connect_items_changed(clone!(@weak self as obj => move |model, _, _, _| {
                obj.set_list_box_visibility(model.upcast_ref());
            }));

            imp.list_box.bind_model(Some(&model), |item| {
                view::ContainerRow::from(item.downcast_ref().unwrap()).upcast()
            });
        }

        imp.container_list.set(value);
        self.notify("container-list");
    }

    fn set_list_box_visibility(&self, model: &gio::ListModel) {
        self.imp().list_box.set_visible(model.n_items() > 0);
    }

    fn update_properties_filter(&self) {
        self.imp()
            .properties_filter
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
