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

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/pod/details-page.ui")]
    pub(crate) struct DetailsPage {
        pub(super) pod: WeakRef<model::Pod>,
        pub(super) handler_id: RefCell<Option<glib::SignalHandlerId>>,
        #[template_child]
        pub(super) leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub(super) back_navigation_controls: TemplateChild<view::BackNavigationControls>,
        #[template_child]
        pub(super) menu_button: TemplateChild<view::PodMenuButton>,
        #[template_child]
        pub(super) id_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) created_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) hostname_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) inspection_row: TemplateChild<adw::PreferencesRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DetailsPage {
        const NAME: &'static str = "PdsPodDetailsPage";
        type Type = super::DetailsPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.add_binding_action(gdk::Key::F10, gdk::ModifierType::empty(), "menu.show", None);
            klass.install_action("menu.show", None, |widget, _, _| {
                widget.show_menu();
            });

            klass.install_action("pod.inspect", None, move |widget, _, _| {
                widget.show_inspection();
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

    impl ObjectImpl for DetailsPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "pod",
                    "Pod",
                    "The pod of this pod details page",
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
            let data_expr = pod_expr.chain_property::<model::Pod>("data");
            let hostname_expr = data_expr.chain_property::<model::PodData>("hostname");

            pod_expr
                .chain_property::<model::Pod>("id")
                .chain_closure::<String>(closure!(|_: glib::Object, id: &str| {
                    id.chars().take(12).collect::<String>()
                }))
                .bind(&*self.id_row, "value", Some(obj));

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

            hostname_expr.bind(&*self.hostname_row, "value", Some(obj));
            hostname_expr
                .chain_closure::<bool>(closure!(
                    |_: glib::Object, hostname: String| !hostname.is_empty()
                ))
                .bind(&*self.hostname_row, "visible", Some(obj));

            data_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, data: Option<model::PodData>| {
                    data.is_none()
                }))
                .bind(&*self.inspection_row, "visible", Some(obj));
        }

        fn dispose(&self, obj: &Self::Type) {
            if let Some(container) = obj.pod() {
                container.disconnect(self.handler_id.take().unwrap());
            }
            self.leaflet.unparent();
        }
    }

    impl WidgetImpl for DetailsPage {}
}

glib::wrapper! {
    pub(crate) struct DetailsPage(ObjectSubclass<imp::DetailsPage>) @extends gtk::Widget;
}

impl From<&model::Pod> for DetailsPage {
    fn from(pod: &model::Pod) -> Self {
        glib::Object::new(&[("pod", pod)]).expect("Failed to create PdsPodDetailsPage")
    }
}

impl DetailsPage {
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
            pod.inspect();
            pod.connect_inspection_failed(clone!(@weak self as obj => move |_| {
                utils::show_toast(&obj, &gettext("Error on loading pod data"));
            }));

            let handler_id = pod.connect_deleted(clone!(@weak self as obj => move |pod| {
                utils::show_toast(&obj, &gettext!("Pod '{}' has been deleted", pod.name()));
                obj.imp().back_navigation_controls.navigate_back();
            }));
            imp.handler_id.replace(Some(handler_id));
        }

        imp.pod.set(value);
        self.notify("pod");
    }

    fn show_inspection(&self) {
        if let Some(pod) = self.pod().as_ref().and_then(model::Pod::api_pod) {
            self.action_set_enabled("pod.inspect", false);
            utils::do_async(
                async move { pod.inspect().await.map_err(anyhow::Error::from) },
                clone!(@weak self as obj => move |result| {
                    obj.action_set_enabled("pod.inspect", true);
                    match result
                        .and_then(|data| view::InspectionPage::new(
                            &gettext("Pod Inspection"), &data
                        ))
                    {
                        Ok(page) => utils::leaflet_overlay(&*obj.imp().leaflet).show_details(&page),
                        Err(e) => utils::show_error_toast(
                            &obj,
                            &gettext("Error on inspecting pod"),
                            &e.to_string()
                        ),
                    }
                }),
            );
        }
    }

    fn show_processes(&self) {
        if let Some(pod) = self.pod() {
            utils::leaflet_overlay(&*self.imp().leaflet).show_details(&view::TopPage::from(&pod));
        }
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
