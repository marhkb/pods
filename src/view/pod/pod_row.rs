use std::cell::RefCell;

use adw::subclass::prelude::ActionRowImpl;
use adw::subclass::prelude::PreferencesRowImpl;
use gtk::glib;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::model::SelectableExt;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/pod-row.ui")]
    pub(crate) struct PodRow {
        pub(super) pod: WeakRef<model::Pod>,
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[template_child]
        pub(super) check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) end_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodRow {
        const NAME: &'static str = "PodRow";
        type Type = super::PodRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("pod-row.activate", None, move |widget, _, _| {
                widget.activate();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "pod",
                    "Pod",
                    "The pod of this PodRow",
                    model::Pod::static_type(),
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
                "pod" => obj.set_pod(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "pod" => obj.pod().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let pod_expr = Self::Type::this_expression("pod");

            let selection_mode_expr = pod_expr
                .chain_property::<model::Pod>("pod-list")
                .chain_property::<model::PodList>("selection-mode");

            selection_mode_expr.bind(&self.check_button.parent().unwrap(), "visible", Some(obj));
            selection_mode_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, is_selection_mode: bool| {
                    !is_selection_mode
                }))
                .bind(&*self.end_box, "visible", Some(obj));

            let status_expr = pod_expr.chain_property::<model::Pod>("status");

            pod_expr
                .chain_property::<model::Pod>("name")
                .chain_closure::<String>(closure!(|_: glib::Object, name: Option<String>| {
                    utils::escape(&utils::format_option(name))
                }))
                .bind(obj, "title", Some(obj));

            pod_expr
                .chain_property::<model::Pod>("id")
                .chain_closure::<String>(closure!(|_: glib::Object, id: &str| {
                    id.chars().take(12).collect::<String>()
                }))
                .bind(obj, "subtitle", Some(obj));

            status_expr
                .chain_closure::<String>(closure!(|_: glib::Object, status: model::PodStatus| {
                    status.to_string()
                }))
                .bind(&*self.status_label, "label", Some(obj));

            let css_classes = self.status_label.css_classes();
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: glib::Object, status: model::PodStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(glib::GString::from(
                                super::super::pod_status_css_class(status),
                            )))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.status_label, "css-classes", Some(obj));
        }
    }

    impl WidgetImpl for PodRow {}
    impl ListBoxRowImpl for PodRow {}
    impl PreferencesRowImpl for PodRow {}
    impl ActionRowImpl for PodRow {}
}

glib::wrapper! {
    pub(crate) struct PodRow(ObjectSubclass<imp::PodRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

impl From<&model::Pod> for PodRow {
    fn from(pod: &model::Pod) -> Self {
        glib::Object::new(&[("pod", pod)]).expect("Failed to create PodRow")
    }
}

impl PodRow {
    pub(crate) fn pod(&self) -> Option<model::Pod> {
        self.imp().pod.upgrade()
    }

    fn set_pod(&self, value: Option<&model::Pod>) {
        if self.pod().as_ref() == value {
            return;
        }

        let imp = self.imp();

        let mut bindings = imp.bindings.borrow_mut();
        while let Some(binding) = bindings.pop() {
            binding.unbind();
        }

        if let Some(pod) = value {
            let binding = pod
                .bind_property("selected", &*imp.check_button, "active")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            bindings.push(binding);
        }

        imp.pod.set(value);
        self.notify("pod");
    }

    fn activate(&self) {
        if let Some(pod) = self.pod().as_ref() {
            if pod
                .pod_list()
                .map(|list| list.is_selection_mode())
                .unwrap_or(false)
            {
                pod.select();
            } else {
                utils::find_leaflet_overlay(self).show_details(&view::PodDetailsPage::from(pod));
            }
        }
    }
}
