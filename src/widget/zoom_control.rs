use std::cell::Cell;
use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::glib;
use gtk::glib::closure;
use gtk::CompositeTemplate;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ZoomControl)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/zoom_control.ui")]
    pub(crate) struct ZoomControl {
        #[property(get, set, minimum = 0.0)]
        pub(super) zoom_factor: Cell<f64>,
        #[template_child]
        pub(super) zoom_out_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) zoom_normal_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) zoom_in_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ZoomControl {
        const NAME: &'static str = "PdsZoomControl";
        type Type = super::ZoomControl;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ZoomControl {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                Self::derived_properties()
                    .iter()
                    .cloned()
                    .chain(vec![
                        glib::ParamSpecString::builder("zoom-out-action-name")
                            .explicit_notify()
                            .build(),
                        glib::ParamSpecString::builder("zoom-normal-action-name")
                            .explicit_notify()
                            .build(),
                        glib::ParamSpecString::builder("zoom-in-action-name")
                            .explicit_notify()
                            .build(),
                    ])
                    .collect::<Vec<_>>()
            })
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "zoom-out-action-name" => self.obj().set_zoom_out_action_name(value.get().unwrap()),
                "zoom-normal-action-name" => {
                    self.obj().set_zoom_normal_action_name(value.get().unwrap())
                }
                "zoom-in-action-name" => self.obj().set_zoom_in_action_name(value.get().unwrap()),
                _ => self.derived_set_property(id, value, pspec),
            }
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "zoom-out-action-name" => self.obj().zoom_out_action_name().to_value(),
                "zoom-normal-action-name" => self.obj().zoom_normal_action_name().to_value(),
                "zoom-in-action-name" => self.obj().zoom_in_action_name().to_value(),
                _ => self.derived_property(id, pspec),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            Self::Type::this_expression("zoom-factor")
                .chain_closure::<String>(closure!(|_: Self::Type, factor: f64| {
                    format!("{:.0}%", 100.0 * factor)
                }))
                .bind(&*self.zoom_normal_button, "label", Some(&*self.obj()));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ZoomControl {}
}

glib::wrapper! {
    pub(crate) struct ZoomControl(ObjectSubclass<imp::ZoomControl>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ZoomControl {
    pub(crate) fn zoom_out_action_name(&self) -> Option<glib::GString> {
        self.imp().zoom_out_button.action_name()
    }

    pub(crate) fn set_zoom_out_action_name(&self, value: Option<&str>) {
        if self.zoom_out_action_name().as_deref() == value {
            return;
        }
        self.imp().zoom_out_button.set_action_name(value);
        self.notify("zoom-out-action-name");
    }

    pub(crate) fn zoom_normal_action_name(&self) -> Option<glib::GString> {
        self.imp().zoom_normal_button.action_name()
    }

    pub(crate) fn set_zoom_normal_action_name(&self, value: Option<&str>) {
        if self.zoom_normal_action_name().as_deref() == value {
            return;
        }
        self.imp().zoom_normal_button.set_action_name(value);
        self.notify("zoom-normal-action-name");
    }

    pub(crate) fn zoom_in_action_name(&self) -> Option<glib::GString> {
        self.imp().zoom_in_button.action_name()
    }

    pub(crate) fn set_zoom_in_action_name(&self, value: Option<&str>) {
        if self.zoom_in_action_name().as_deref() == value {
            return;
        }
        self.imp().zoom_in_button.set_action_name(value);
        self.notify("zoom-in-action-name");
    }
}
