use adw::traits::BinExt;
use gettextrs::gettext;
use gettextrs::ngettext;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;

use crate::model;
use crate::utils;
use crate::view;
use crate::window::Window;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/pods-panel.ui")]
    pub(crate) struct PodsPanel {
        pub(super) settings: utils::PodsSettings,
        pub(super) pod_list: WeakRef<model::PodList>,
        pub(super) properties_filter: OnceCell<gtk::Filter>,
        pub(super) sorter: OnceCell<gtk::Sorter>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) pods_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) show_only_running_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodsPanel {
        const NAME: &'static str = "PodsPanel";
        type Type = super::PodsPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                "pods.create",
                None,
            );
            klass.install_action("pods.create", None, move |widget, _, _| {
                widget.create_pod();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodsPanel {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "pod-list",
                    "Pod List",
                    "The list of pods",
                    model::PodList::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                )]
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
                "pod-list" => obj.set_pod_list(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "pod-list" => obj.pod_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            self.settings.connect_changed(
                Some("show-only-running-pods"),
                clone!(@weak obj => move |_, _| obj.update_properties_filter()),
            );
            self.settings
                .bind(
                    "show-only-running-pods",
                    &*self.show_only_running_switch,
                    "active",
                )
                .build();

            let pod_list_expr = Self::Type::this_expression("pod-list");
            let pod_list_len_expr = pod_list_expr.chain_property::<model::PodList>("len");

            gtk::ClosureExpression::new::<String, _, _>(
                &[
                    pod_list_len_expr.as_ref(),
                    pod_list_expr
                        .chain_property::<model::PodList>("listing")
                        .as_ref(),
                ],
                closure!(|_: Self::Type, len: u32, listing: bool| {
                    if len == 0 && listing {
                        "spinner"
                    } else {
                        "pods"
                    }
                }),
            )
            .bind(&*self.main_stack, "visible-child-name", Some(obj));

            gtk::ClosureExpression::new::<Option<String>, _, _>(
                &[
                    pod_list_len_expr,
                    pod_list_expr.chain_property::<model::PodList>("running"),
                ],
                closure!(|_: Self::Type, len: u32, running: u32| {
                    if len == 0 {
                        gettext("No pods found")
                    } else if len == 1 {
                        if running == 1 {
                            gettext("1 pod, running")
                        } else {
                            gettext("1 pod, stopped")
                        }
                    } else {
                        ngettext!(
                            // Translators: There's a wide space (U+2002) between ", {}".
                            "{} pod total, {} running",
                            "{} pods total, {} running",
                            len,
                            len,
                            running,
                        )
                    }
                }),
            )
            .bind(&*self.pods_group, "description", Some(obj));

            let properties_filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    !obj.imp().show_only_running_switch.is_active() ||
                        item.downcast_ref::<model::Pod>().unwrap().status()
                            == model::PodStatus::Running
                }));

            let sorter = gtk::CustomSorter::new(|obj1, obj2| {
                let pod1 = obj1.downcast_ref::<model::Pod>().unwrap();
                let pod2 = obj2.downcast_ref::<model::Pod>().unwrap();

                pod1.name().cmp(&pod2.name()).into()
            });

            self.properties_filter
                .set(properties_filter.upcast())
                .unwrap();
            self.sorter.set(sorter.upcast()).unwrap();
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.main_stack.unparent();
        }
    }

    impl WidgetImpl for PodsPanel {}
}

glib::wrapper! {
    pub(crate) struct PodsPanel(ObjectSubclass<imp::PodsPanel>)
        @extends gtk::Widget;
}

impl Default for PodsPanel {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create PodsPanel")
    }
}

impl PodsPanel {
    pub(crate) fn pod_list(&self) -> Option<model::PodList> {
        self.imp().pod_list.upgrade()
    }

    pub(crate) fn set_pod_list(&self, value: &model::PodList) {
        if self.pod_list().as_ref() == Some(value) {
            return;
        }

        // TODO: For multi-client: Figure out whether signal handlers need to be disconnected.
        let imp = self.imp();

        value.connect_notify_local(
            Some("running"),
            clone!(@weak self as obj => move |_ ,_| obj.update_properties_filter()),
        );

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
            view::PodRow::from(item.downcast_ref().unwrap()).upcast()
        });

        imp.pod_list.set(Some(value));
        self.notify("pod-list");
    }

    fn set_list_box_visibility(&self, model: &gio::ListModel) {
        self.imp().list_box.set_visible(model.n_items() > 0);
    }

    pub(crate) fn update_properties_filter(&self) {
        self.imp()
            .properties_filter
            .get()
            .unwrap()
            .changed(gtk::FilterChange::Different);
    }

    fn create_pod(&self) {
        let leaflet_overlay = utils::find_leaflet_overlay(self);

        if leaflet_overlay.child().is_none() {
            leaflet_overlay.show_details(&view::PodCreationPage::from(
                self.root()
                    .unwrap()
                    .downcast::<Window>()
                    .unwrap()
                    .connection_manager()
                    .client()
                    .as_ref(),
            ));
        }
    }
}
