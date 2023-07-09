use std::cell::Cell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::glib;
use sourceview5::subclass::view::ViewImpl;

use crate::utils::PodsSettings;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::ScalableTextView)]
    pub(crate) struct ScalableTextView {
        pub(super) settings: PodsSettings,
        pub(super) css_provider: gtk::CssProvider,
        #[property(get, set, minimum = 0.1, lax_validation, default = 1.0)]
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
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
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

    impl WidgetImpl for ScalableTextView {}
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
        self.imp().css_provider.load_from_data(&format!(
            "textview {{ font-size: {}em; }}",
            self.font_scale()
        ));
    }
}
