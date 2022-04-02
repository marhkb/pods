use std::cell::Cell;

use cascade::cascade;
use gettextrs::gettext;
use gtk::glib::{clone, closure, WeakRef};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, CompositeTemplate};
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::{model, utils, view};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/containers-panel.ui")]
    pub(crate) struct ContainersPanel {
        pub(super) settings: utils::PodsSettings,
        pub(super) container_list: WeakRef<model::ContainerList>,
        pub(super) show_only_running: Cell<bool>,
        pub(super) properties_filter: OnceCell<gtk::Filter>,
        pub(super) search_filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) progress_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) containers_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainersPanel {
        const NAME: &'static str = "ContainersPanel";
        type Type = super::ContainersPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainersPanel {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "container-list",
                        "Container List",
                        "The list of containers",
                        model::ContainerList::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecBoolean::new(
                        "show-only-running",
                        "Show-Only-Running",
                        "Whether to show only running containers",
                        true,
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
                "container-list" => obj.set_container_list(value.get().unwrap()),
                "show-only-running" => obj.set_show_only_running(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container-list" => obj.container_list().to_value(),
                "show-only-running" => obj.show_only_running().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let container_list_expr = Self::Type::this_expression("container-list");
            let container_list_len_expr =
                container_list_expr.chain_property::<model::ContainerList>("len");
            let fetched_expr =
                container_list_expr.chain_property::<model::ContainerList>("fetched");
            let fetching_exprs = &[
                fetched_expr.clone(),
                container_list_expr.chain_property::<model::ContainerList>("to-fetch"),
            ];

            gtk::ClosureExpression::new::<gtk::Widget, _, _>(
                &[
                    container_list_len_expr,
                    container_list_expr.chain_property::<model::ContainerList>("listing"),
                ],
                closure!(|obj: Self::Type, len: u32, listing: bool| {
                    let imp = obj.imp();
                    if len == 0 && listing {
                        imp.spinner.upcast_ref::<gtk::Widget>().clone()
                    } else {
                        imp.overlay.upcast_ref::<gtk::Widget>().clone()
                    }
                }),
            )
            .bind(&*self.main_stack, "visible-child", Some(obj));

            gtk::ClosureExpression::new::<f64, _, _>(
                fetching_exprs,
                closure!(|_: glib::Object, fetched: u32, to_fetch: u32| {
                    f64::min(1.0, fetched as f64 / to_fetch as f64)
                }),
            )
            .bind(&*self.progress_bar, "fraction", Some(obj));

            gtk::ClosureExpression::new::<String, _, _>(
                fetching_exprs,
                closure!(|_: glib::Object, fetched: u32, to_fetch: u32| {
                    if fetched >= to_fetch {
                        "empty"
                    } else {
                        "bar"
                    }
                }),
            )
            .bind(&*self.progress_stack, "visible-child-name", Some(obj));

            gtk::Stack::this_expression("visible-child-name")
                .chain_closure::<u32>(closure!(|_: glib::Object, name: &str| {
                    match name {
                        "empty" => 0_u32,
                        "bar" => 1000,
                        _ => unreachable!(),
                    }
                }))
                .bind(
                    &*self.progress_stack,
                    "transition-duration",
                    Some(&*self.progress_stack),
                );

            gtk::ClosureExpression::new::<String, _, _>(
                &[container_list_expr, fetched_expr],
                closure!(
                    |_: glib::Object, list: Option<model::ContainerList>, _: u32| {
                        match list.filter(|list| list.len() > 0) {
                            Some(list) => gettext!(
                                // Translators: There's a wide space (U+2002) between ", {}".
                                "{} Containers total, {} running",
                                list.len(),
                                list.count(model::ContainerStatus::Running),
                            ),
                            None => gettext("No containers found"),
                        }
                    }
                ),
            )
            .bind(&*self.containers_group, "description", Some(obj));

            let properties_filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    !obj.show_only_running() ||
                        item.downcast_ref::<model::Container>().unwrap().status()
                            == model::ContainerStatus::Running
                }));

            obj.connect_notify_local(
                Some("show-only-running"),
                clone!(@weak obj => move |_ ,_| obj.update_properties_filter()),
            );

            let search_filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    let container = item
                        .downcast_ref::<model::Container>()
                        .unwrap();
                    let query = obj.imp().search_entry.text();
                    let query = query.as_str();

                    container.name().map(|name| name.contains(query)).unwrap_or(false)
                }));

            self.search_entry
                .connect_search_changed(clone!(@weak obj => move |_| obj.update_search_filter()));

            let sorter = gtk::CustomSorter::new(|obj1, obj2| {
                let container1 = obj1.downcast_ref::<model::Container>().unwrap();
                let container2 = obj2.downcast_ref::<model::Container>().unwrap();

                container1.name().cmp(&container2.name()).into()
            });

            self.properties_filter
                .set(properties_filter.upcast())
                .unwrap();
            self.search_filter.set(search_filter.upcast()).unwrap();
            self.sorter.set(sorter.upcast()).unwrap();

            self.settings
                .bind("show-only-running-containers", obj, "show-only-running")
                .build();
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.main_stack.unparent();
        }
    }

    impl WidgetImpl for ContainersPanel {
        fn root(&self, widget: &Self::Type) {
            self.parent_root(widget);
            self.search_bar
                .set_key_capture_widget(widget.root().as_ref());
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainersPanel(ObjectSubclass<imp::ContainersPanel>)
        @extends gtk::Widget;
}

impl Default for ContainersPanel {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create ContainersPanel")
    }
}

impl ContainersPanel {
    pub(crate) fn container_list(&self) -> Option<model::ContainerList> {
        self.imp().container_list.upgrade()
    }

    pub(crate) fn set_container_list(&self, value: &model::ContainerList) {
        if self.container_list().as_ref() == Some(value) {
            return;
        }

        // TODO: For multi-client: Figure out whether signal handlers need to be disconnected.
        value.connect_notify_local(
            Some("fetched"),
            clone!(@weak self as obj => move |_ ,_| {
                obj.update_properties_filter();
                obj.update_search_filter();
                obj.update_sorter();
            }),
        );

        let imp = self.imp();

        let model = gtk::SortListModel::new(
            Some(&gtk::FilterListModel::new(
                Some(&gtk::FilterListModel::new(
                    Some(value),
                    imp.search_filter.get(),
                )),
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

        imp.container_list.set(Some(value));
        self.notify("container-list");
    }

    pub(crate) fn show_only_running(&self) -> bool {
        self.imp().show_only_running.get()
    }

    pub(crate) fn set_show_only_running(&self, value: bool) {
        if self.show_only_running() == value {
            return;
        }
        self.imp().show_only_running.set(value);
        self.notify("show-only-running");
    }

    fn set_list_box_visibility(&self, model: &gio::ListModel) {
        self.imp().list_box.set_visible(model.n_items() > 0);
    }

    pub(crate) fn connect_search_button(&self, search_button: &gtk::ToggleButton) {
        search_button
            .bind_property("active", &*self.imp().search_bar, "search-mode-enabled")
            .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
            .build();
    }

    pub(crate) fn toggle_search(&self) {
        let imp = self.imp();
        if imp.search_bar.is_search_mode() {
            imp.search_bar.set_search_mode(false);
        } else {
            imp.search_bar.set_search_mode(true);
            imp.search_entry.grab_focus();
        }
    }

    pub(crate) fn update_properties_filter(&self) {
        self.imp()
            .properties_filter
            .get()
            .unwrap()
            .changed(gtk::FilterChange::Different);
    }

    pub(crate) fn update_search_filter(&self) {
        self.imp()
            .search_filter
            .get()
            .unwrap()
            .changed(gtk::FilterChange::Different);
    }

    pub(crate) fn update_sorter(&self) {
        self.imp()
            .sorter
            .get()
            .unwrap()
            .changed(gtk::SorterChange::Different);
    }
}

pub(crate) fn menu() -> gio::Menu {
    cascade! {
        gio::Menu::new();
        ..append_section(None, &cascade!{
            gio::Menu::new();
            ..append(Some(&gettext("_Show only running containers")), Some("containers.show-only-running"));
        });
    }
}
