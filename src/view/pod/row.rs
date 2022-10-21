use std::cell::RefCell;

use adw::subclass::prelude::ActionRowImpl;
use adw::subclass::prelude::PreferencesRowImpl;
use gtk::glib;
use gtk::glib::closure;
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
    #[template(resource = "/com/github/marhkb/Pods/ui/pod/row.ui")]
    pub(crate) struct Row {
        pub(super) pod: glib::WeakRef<model::Pod>,
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[template_child]
        pub(super) status_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) end_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsPodRow";
        type Type = super::Row;
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

    impl ObjectImpl for Row {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::Pod>("pod")
                    .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "pod" => self.instance().set_pod(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "pod" => self.instance().pod().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            let pod_expr = Self::Type::this_expression("pod");

            let selection_mode_expr = pod_expr
                .chain_property::<model::Pod>("pod-list")
                .chain_property::<model::PodList>("selection-mode");

            selection_mode_expr.bind(&*self.check_button, "visible", Some(obj));
            selection_mode_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, is_selection_mode: bool| {
                    !is_selection_mode
                }))
                .bind(&*self.end_box, "visible", Some(obj));

            let status_expr = pod_expr.chain_property::<model::Pod>("status");

            status_expr
                .chain_closure::<String>(closure!(|_: Self::Type, status: model::PodStatus| {
                    match status {
                        model::PodStatus::Running => "media-playback-start-symbolic",
                        model::PodStatus::Paused => "media-playback-pause-symbolic",
                        model::PodStatus::Degraded => "degraded-pod-symbolic",
                        _ => "media-playback-stop-symbolic",
                    }
                }))
                .bind(&*self.status_image, "icon-name", Some(obj));

            let css_classes = self.status_image.css_classes();
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::PodStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(glib::GString::from(
                                super::super::pod_status_css_class(status),
                            )))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.status_image, "css-classes", Some(obj));

            gtk::ClosureExpression::new::<String>(
                &[
                    pod_expr.chain_property::<model::Pod>("name"),
                    pod_expr.chain_property::<model::Pod>("to-be-deleted"),
                ],
                closure!(|_: Self::Type, name: &str, to_be_deleted: bool| {
                    let title = utils::escape(name);
                    if to_be_deleted {
                        format!("<s>{title}</s>")
                    } else {
                        title
                    }
                }),
            )
            .bind(obj, "title", Some(obj));

            pod_expr
                .chain_property::<model::Pod>("id")
                .chain_closure::<String>(closure!(|_: Self::Type, id: &str| {
                    id.chars().take(12).collect::<String>()
                }))
                .bind(obj, "subtitle", Some(obj));
        }
    }

    impl WidgetImpl for Row {}
    impl ListBoxRowImpl for Row {}
    impl PreferencesRowImpl for Row {}
    impl ActionRowImpl for Row {}
}

glib::wrapper! {
    pub(crate) struct Row(ObjectSubclass<imp::Row>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::Actionable, gtk::ConstraintTarget;
}

impl From<&model::Pod> for Row {
    fn from(pod: &model::Pod) -> Self {
        glib::Object::builder::<Self>().property("pod", pod).build()
    }
}

impl Row {
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
