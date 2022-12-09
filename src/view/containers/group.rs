use std::cell::RefCell;

use adw::subclass::prelude::PreferencesGroupImpl;
use gettextrs::gettext;
use gettextrs::ngettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
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
    #[template(resource = "/com/github/marhkb/Pods/ui/containers/group.ui")]
    pub(crate) struct Group {
        pub(super) settings: utils::PodsSettings,
        pub(super) container_list: glib::WeakRef<model::AbstractContainerList>,
        pub(super) no_containers_label: RefCell<Option<String>>,
        pub(super) show_running_settings_key: RefCell<String>,
        pub(super) properties_filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        #[template_child]
        pub(super) create_container_row: TemplateChild<gtk::ListBoxRow>,
        #[template_child]
        pub(super) header_suffix_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) show_only_running_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) create_container_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Group {
        const NAME: &'static str = "PdsContainersGroup";
        type Type = super::Group;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Group {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::builder("no-containers-label")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecString::builder("show-running-settings-key")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecObject::builder::<model::AbstractContainerList>(
                        "container-list",
                    )
                    .explicit_notify()
                    .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "no-containers-label" => obj.set_no_containers_label(value.get().unwrap()),
                "show-running-settings-key" => {
                    obj.set_show_running_settings_key(value.get().unwrap_or_default());
                }
                "container-list" => obj.set_container_list(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "no-containers-label" => obj.no_containers_label().to_value(),
                "show-running-settings-key" => obj.show_running_settings_key().to_value(),
                "container-list" => obj.container_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let container_list_expr = Self::Type::this_expression("container-list");
            let container_list_len_expr =
                container_list_expr.chain_property::<model::AbstractContainerList>("len");
            let is_selection_mode_expr = container_list_expr
                .chain_property::<model::ContainerList>("selection-mode")
                .chain_closure::<bool>(closure!(|_: Self::Type, selection_mode: bool| {
                    !selection_mode
                }));

            container_list_len_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, len: u32| len > 0))
                .bind(&*self.header_suffix_box, "visible", Some(obj));

            is_selection_mode_expr.bind(&*self.create_container_button, "visible", Some(obj));
            is_selection_mode_expr.bind(&*self.create_container_row, "visible", Some(obj));

            gtk::ClosureExpression::new::<Option<String>>(
                &[
                    container_list_len_expr,
                    container_list_expr.chain_property::<model::AbstractContainerList>("running"),
                ],
                closure!(|obj: Self::Type, len: u32, running: u32| {
                    if len == 0 {
                        obj.no_containers_label()
                    } else {
                        Some(if len == 1 {
                            if running == 1 {
                                gettext("1 container, running")
                            } else {
                                gettext("1 container, stopped")
                            }
                        } else {
                            ngettext!(
                                // Translators: There's a wide space (U+2002) between ", {}".
                                "{} container total, {} running",
                                "{} containers total, {} running",
                                len,
                                len,
                                running,
                            )
                        })
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

    impl WidgetImpl for Group {}
    impl PreferencesGroupImpl for Group {}
}

glib::wrapper! {
    pub(crate) struct Group(ObjectSubclass<imp::Group>)
        @extends gtk::Widget, adw::PreferencesGroup,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Group {
    fn default() -> Self {
        glib::Object::builder::<Self>().build()
    }
}

impl Group {
    pub(crate) fn action_create_container() -> &'static str {
        "containers-group.create-container"
    }

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

            imp.list_box.bind_model(Some(&model), |item| {
                view::ContainerRow::from(item.downcast_ref().unwrap()).upcast()
            });
            imp.list_box.append(&*imp.create_container_row);
        }

        imp.container_list.set(value);
        self.notify("container-list");
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
