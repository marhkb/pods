use std::cell::Cell;

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::sync::Lazy;
use sourceview5::subclass::view::ViewImpl;

use crate::utils::PodsSettings;

const MIN_FONT_SCALE: f64 = 0.1;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct ScalableTextView {
        pub(super) settings: PodsSettings,
        pub(super) css_provider: gtk::CssProvider,
        pub(super) font_scale: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ScalableTextView {
        const NAME: &'static str = "PdsScalableTextView";
        type Type = super::ScalableTextView;
        type ParentType = sourceview5::View;
    }

    impl ObjectImpl for ScalableTextView {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecDouble::builder("font-scale")
                    .explicit_notify()
                    .minimum(MIN_FONT_SCALE)
                    .default_value(1.0)
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "font-scale" => self.obj().set_font_scale(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "font-scale" => self.obj().font_scale().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            obj.style_context()
                .add_provider(&self.css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
            obj.connect_notify_local(Some("font-scale"), |obj, _| {
                obj.update_css();
            });

            self.settings
                .bind("text-view-font-scale", obj, "font-scale")
                .build();
        }
    }

    impl WidgetImpl for ScalableTextView {
        fn realize(&self) {
            self.parent_realize();
        }
    }

    impl TextViewImpl for ScalableTextView {}
    impl ViewImpl for ScalableTextView {}
}

glib::wrapper! {
    pub(crate) struct ScalableTextView(ObjectSubclass<imp::ScalableTextView>)
        @extends gtk::Widget, gtk::TextView, sourceview5::View,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Scrollable;
}

impl Default for ScalableTextView {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl ScalableTextView {
    pub(crate) fn font_scale(&self) -> f64 {
        self.imp().font_scale.get()
    }

    pub(crate) fn set_font_scale(&self, value: f64) {
        if self.font_scale() == value || value < MIN_FONT_SCALE {
            return;
        }
        self.imp().font_scale.set(value);
        self.notify("font-scale");
    }

    pub(crate) fn zoom_in(&self) {
        self.set_font_scale(self.font_scale() + 0.1);
    }

    pub(crate) fn zoom_out(&self) {
        self.set_font_scale(self.font_scale() - 0.1);
    }

    pub(crate) fn zoom_normal(&self) {
        self.set_font_scale(1.0);
    }

    fn update_css(&self) {
        self.imp().css_provider.load_from_data(
            format!("textview {{ font-size: {}em; }}", self.font_scale()).as_bytes(),
        );
    }
}
