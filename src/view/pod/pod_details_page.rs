use std::cell::RefCell;

use adw::traits::BinExt;
use gettextrs::gettext;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;
use crate::view;
use crate::window::Window;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/pod-details-page.ui")]
    pub(crate) struct PodDetailsPage {
        pub(super) pod: WeakRef<model::Pod>,
        pub(super) handler_id: RefCell<Option<glib::SignalHandlerId>>,
        #[template_child]
        pub(super) menu_button: TemplateChild<view::PodMenuButton>,
        #[template_child]
        pub(super) leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub(super) id_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) hostname_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) created_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodDetailsPage {
        const NAME: &'static str = "PodDetailsPage";
        type Type = super::PodDetailsPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.add_binding_action(gdk::Key::F10, gdk::ModifierType::empty(), "menu.show", None);
            klass.install_action("menu.show", None, |widget, _, _| {
                widget.show_menu();
            });

            klass.install_action("navigation.go-first", None, move |widget, _, _| {
                widget.navigate_to_first();
            });
            klass.install_action("navigation.back", None, move |widget, _, _| {
                widget.navigate_back();
            });

            klass.install_action("pod.show-processes", None, move |widget, _, _| {
                widget.show_processes();
            });

            // For displaying a mnemonic.
            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                "pod.create-container",
                None,
            );

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                "containers.create",
                None,
            );
            klass.install_action("containers.create", None, move |widget, _, _| {
                widget.create_container();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodDetailsPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "pod",
                    "Pod",
                    "The pod of this PodDetailsPage",
                    model::Pod::static_type(),
                    glib::ParamFlags::READWRITE
                        | glib::ParamFlags::CONSTRUCT
                        | glib::ParamFlags::EXPLICIT_NOTIFY,
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
            let status_expr = pod_expr.chain_property::<model::Pod>("status");

            pod_expr
                .chain_property::<model::Pod>("id")
                .chain_closure::<String>(closure!(|_: glib::Object, id: &str| {
                    id.chars().take(12).collect::<String>()
                }))
                .bind(&*self.id_row, "value", Some(obj));

            pod_expr
                .chain_property::<model::Pod>("hostname")
                .chain_closure::<bool>(closure!(|_: glib::Object, hostname: &str| {
                    !hostname.is_empty()
                }))
                .bind(&*self.hostname_row, "visible", Some(obj));

            pod_expr
                .chain_property::<model::Pod>("created")
                .chain_closure::<String>(closure!(|_: glib::Object, created: i64| {
                    glib::DateTime::from_unix_local(created)
                        .unwrap()
                        .format(
                            // Translators: This is a date time format (https://valadoc.org/glib-2.0/GLib.DateTime.format.html)
                            &gettext("%x %X"),
                        )
                        .unwrap()
                }))
                .bind(&*self.created_row, "value", Some(obj));

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

        fn dispose(&self, obj: &Self::Type) {
            if let Some(container) = obj.pod() {
                container.disconnect(self.handler_id.take().unwrap());
            }
            self.leaflet.unparent();
        }
    }

    impl WidgetImpl for PodDetailsPage {
        fn realize(&self, widget: &Self::Type) {
            self.parent_realize(widget);

            widget.action_set_enabled(
                "navigation.go-first",
                widget.previous_leaflet_overlay() != widget.root_leaflet_overlay(),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct PodDetailsPage(ObjectSubclass<imp::PodDetailsPage>) @extends gtk::Widget;
}

impl From<&model::Pod> for PodDetailsPage {
    fn from(pod: &model::Pod) -> Self {
        glib::Object::new(&[("pod", pod)]).expect("Failed to create PodDetailsPage")
    }
}

impl PodDetailsPage {
    fn show_menu(&self) {
        let imp = self.imp();
        if utils::leaflet_overlay(&imp.leaflet).child().is_none() {
            imp.menu_button.popup();
        }
    }
    fn pod(&self) -> Option<model::Pod> {
        self.imp().pod.upgrade()
    }

    fn set_pod(&self, value: Option<&model::Pod>) {
        if self.pod().as_ref() == value {
            return;
        }

        let imp = self.imp();

        if let Some(pod) = self.pod() {
            pod.disconnect(imp.handler_id.take().unwrap());
        }

        if let Some(pod) = value {
            let handler_id = pod.connect_deleted(clone!(@weak self as obj => move |pod| {
                utils::show_toast(&obj, &gettext!("Pod '{}' has been deleted", pod.name()));
                obj.navigate_back();
            }));
            imp.handler_id.replace(Some(handler_id));
        }

        imp.pod.set(value);
        self.notify("pod");
    }

    fn show_processes(&self) {
        if let Some(pod) = self.pod() {
            utils::leaflet_overlay(&*self.imp().leaflet).show_details(&view::TopPage::from(&pod));
        }
    }

    fn navigate_to_first(&self) {
        self.root_leaflet_overlay().hide_details();
    }

    fn navigate_back(&self) {
        self.previous_leaflet_overlay().hide_details();
    }

    fn previous_leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::find_parent_leaflet_overlay(self)
    }

    fn root_leaflet_overlay(&self) -> view::LeafletOverlay {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .leaflet_overlay()
    }

    fn create_container(&self) {
        let imp = self.imp();

        if utils::leaflet_overlay(&*imp.leaflet).child().is_none() {
            imp.menu_button
                .activate_action("pod.create-container", None)
                .unwrap();
        }
    }
}
