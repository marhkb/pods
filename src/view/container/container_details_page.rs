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
    #[template(resource = "/com/github/marhkb/Pods/ui/container-details-page.ui")]
    pub(crate) struct ContainerDetailsPage {
        pub(super) container: WeakRef<model::Container>,
        pub(super) handler_id: RefCell<Option<glib::SignalHandlerId>>,
        #[template_child]
        pub(super) menu_button: TemplateChild<view::ContainerMenuButton>,
        #[template_child]
        pub(super) leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub(super) resources_quick_reference_group:
            TemplateChild<view::ContainerResourcesQuickReferenceGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerDetailsPage {
        const NAME: &'static str = "ContainerDetailsPage";
        type Type = super::ContainerDetailsPage;
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

            klass.install_action("container.inspect", None, move |widget, _, _| {
                widget.show_inspection();
            });
            klass.install_action("container.show-log", None, move |widget, _, _| {
                widget.show_log();
            });
            klass.install_action("container.show-processes", None, move |widget, _, _| {
                widget.show_processes();
            });

            add_binding_action(
                klass,
                gdk::Key::F10,
                gdk::ModifierType::SHIFT_MASK,
                "container.start",
            );

            add_binding_action(
                klass,
                gdk::Key::F2,
                gdk::ModifierType::CONTROL_MASK,
                "container.stop",
            );

            add_binding_action(
                klass,
                gdk::Key::F5,
                gdk::ModifierType::CONTROL_MASK,
                "container.restart",
            );

            add_binding_action(
                klass,
                gdk::Key::F6,
                gdk::ModifierType::SHIFT_MASK,
                "container.rename",
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerDetailsPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "container",
                    "Container",
                    "The container of this ContainerDetailsPage",
                    model::Container::static_type(),
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
                "container" => obj.set_container(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container" => obj.container().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            Self::Type::this_expression("container")
                .chain_property::<model::Container>("status")
                .chain_closure::<bool>(closure!(
                    |_: glib::Object, status: model::ContainerStatus| status
                        == model::ContainerStatus::Running
                ))
                .bind(&*self.resources_quick_reference_group, "visible", Some(obj));
        }

        fn dispose(&self, obj: &Self::Type) {
            if let Some(container) = obj.container() {
                container.disconnect(self.handler_id.take().unwrap());
            }
            self.leaflet.unparent();
        }
    }

    impl WidgetImpl for ContainerDetailsPage {
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
    pub(crate) struct ContainerDetailsPage(ObjectSubclass<imp::ContainerDetailsPage>) @extends gtk::Widget;
}

impl From<&model::Container> for ContainerDetailsPage {
    fn from(image: &model::Container) -> Self {
        glib::Object::new(&[("container", image)]).expect("Failed to create ContainerDetailsPage")
    }
}

impl ContainerDetailsPage {
    fn show_menu(&self) {
        let imp = self.imp();
        if utils::leaflet_overlay(&imp.leaflet).child().is_none() {
            imp.menu_button.popup();
        }
    }

    fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    fn set_container(&self, value: Option<&model::Container>) {
        if self.container().as_ref() == value {
            return;
        }

        let imp = self.imp();

        if let Some(container) = self.container() {
            container.disconnect(imp.handler_id.take().unwrap());
        }

        if let Some(container) = value {
            let handler_id = container.connect_deleted(clone!(@weak self as obj => move |container| {
                utils::show_toast(&obj, &gettext!("Container '{}' has been deleted", container.name()));
                obj.navigate_back();
            }));
            imp.handler_id.replace(Some(handler_id));
        }

        imp.container.set(value);
        self.notify("container");
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

    fn show_inspection(&self) {
        if let Some(container) = self
            .container()
            .as_ref()
            .and_then(model::Container::api_container)
        {
            self.action_set_enabled("container.inspect", false);
            utils::do_async(
                async move { container.inspect().await.map_err(anyhow::Error::from) },
                clone!(@weak self as obj => move |result| {
                    obj.action_set_enabled("container.inspect", true);
                    match result
                        .and_then(|data| view::InspectionPage::new(
                            &gettext("Container Inspection"), &data
                        ))
                    {
                        Ok(page) => utils::leaflet_overlay(&*obj.imp().leaflet).show_details(&page),
                        Err(e) => utils::show_error_toast(
                            &obj,
                            &gettext("Error on inspecting container"),
                            &e.to_string()
                        ),
                    }
                }),
            );
        }
    }

    fn show_log(&self) {
        if let Some(container) = self.container() {
            utils::leaflet_overlay(&*self.imp().leaflet)
                .show_details(&view::ContainerLogPage::from(&container));
        }
    }

    fn show_processes(&self) {
        if let Some(container) = self.container() {
            utils::leaflet_overlay(&*self.imp().leaflet)
                .show_details(&view::TopPage::from(&container));
        }
    }
}

fn add_binding_action(
    klass: &mut <imp::ContainerDetailsPage as ObjectSubclass>::Class,
    keyval: gdk::Key,
    mods: gdk::ModifierType,
    action: &'static str,
) {
    klass.add_binding(
        keyval,
        mods,
        |widget, _| {
            let imp = widget.imp();
            match utils::leaflet_overlay(&imp.leaflet).child() {
                None => imp.menu_button.activate_action(action, None).is_ok(),
                Some(_) => false,
            }
        },
        None,
    );

    // For displaying a mnemonic.
    klass.add_binding_action(keyval, mods, action, None);
}
