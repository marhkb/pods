// Inspired by https://github.com/phastmike/vala-circular-progress-bar/blob/1528d42a6045734038bf0022a88b846edf582b3a/circular-progress-bar.vala.

use std::cell::Cell;
use std::f64;

use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::ParamSpecBuilderExt;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/component/circular-progress-bar.ui")]
    pub(crate) struct CircularProgressBar {
        pub(super) percentage: Cell<f64>,
        #[template_child]
        pub(super) overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) drawing_area: TemplateChild<gtk::DrawingArea>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CircularProgressBar {
        const NAME: &'static str = "PdsCircularProgressBar";
        type Type = super::CircularProgressBar;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CircularProgressBar {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecDouble::builder("percentage")
                        .maximum(1.0)
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                    glib::ParamSpecString::builder("icon-name")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.instance();
            match pspec.name() {
                "percentage" => obj.set_percentage(value.get().unwrap()),
                "icon-name" => obj.set_icon_name(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.instance();
            match pspec.name() {
                "percentage" => obj.percentage().to_value(),
                "icon-name" => obj.icon_name().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            self.icon.connect_notify_local(
                Some("icon-name"),
                clone!(@weak obj => move |_, _| obj.notify("icon-name")),
            );

            self.drawing_area
                .set_draw_func(clone!(@weak obj => move |_, cr, w, h| {
                    let style_manager = adw::StyleManager::default();

                    let alpha = if style_manager.is_high_contrast() {
                        (0.33, 1.0)
                    } else {
                        (0.15, 0.75)
                    };

                    let colors = if style_manager.is_dark() {
                        [
                            // background: @view_bg_color
                            (0.188, 0.188, 0.188, 1.0),
                            // @borders
                            (1.0, 1.0, 1.0, alpha.0),
                            // @accent_color
                            (0.470, 0.682, 0.929, alpha.1),
                            // @warning_color
                            (0.972, 0.894, 0.360, alpha.1),
                            // @error_color
                            (1.0, 0.482, 0.388, alpha.1),
                        ]
                    } else {
                        [
                            // background: @window_bg_color
                            (0.98, 0.98, 0.98, 1.0),
                            // @borders
                            (0.0, 0.0, 0.0, alpha.0),
                            // @accent_color
                            (0.109, 0.443, 0.847, alpha.1),
                            // @warning_color
                            (0.682, 0.482, 0.011, alpha.1),
                            // @error_color
                            (0.752, 0.109, 0.156, alpha.1),
                        ]
                    };

                    let pi = f64::consts::PI;

                    cr.save().unwrap();

                    let center_x = w as f64 / 2.0;
                    let center_y = h as f64 / 2.0;
                    let radius = f64::min(center_x, center_y);

                    cr.set_line_cap(gdk::cairo::LineCap::Butt);

                    // Radius Fill
                    let line_width_fill = 1.0;
                    let delta_fill = radius - (line_width_fill / 2.0) - 1.0;

                    cr.arc(center_x, center_y, delta_fill, 0.0, 2. * pi);
                    cr.set_source_rgba(colors[0].0, colors[0].1, colors[0].2, colors[0].3);
                    cr.fill().unwrap();

                    cr.set_line_width(line_width_fill);
                    cr.arc(center_x, center_y, delta_fill, 0.0, 2. * pi);
                    cr.set_source_rgba(colors[1].0, colors[1].1, colors[1].2, colors[1].3);
                    cr.stroke().unwrap();

                    // Percentage
                    let line_width_percentage = 3.0;
                    let delta_percentage = radius - (line_width_percentage / 2.0);

                    let percentage = obj.percentage();
                    if percentage < 0.8 {
                        cr.set_source_rgba(colors[2].0, colors[2].1, colors[2].2, colors[2].3);
                    } else if percentage < 0.95 {
                        cr.set_source_rgba(colors[3].0, colors[3].1, colors[3].2, colors[3].3);
                    } else {
                        cr.set_source_rgba(colors[4].0, colors[4].1, colors[4].2, colors[4].3);
                    }

                    cr.set_line_width(line_width_percentage);
                    cr.arc(
                        center_x,
                        center_y,
                        delta_percentage,
                        1.5 * pi,
                        (1.5 + percentage * 2.0) * pi,
                    );
                    cr.stroke().unwrap();

                    cr.arc(
                        center_x,
                        center_y,
                        delta_percentage,
                        1.5 * pi,
                        (1.5 + percentage * 2.0) * pi,
                    );

                    cr.restore().unwrap();
                }));

            adw::StyleManager::default().connect_dark_notify(clone!(@weak obj => move |_| {
                obj.imp().drawing_area.queue_draw();
            }));
            adw::StyleManager::default().connect_high_contrast_notify(
                clone!(@weak obj => move |_| {
                    obj.imp().drawing_area.queue_draw();
                }),
            );
        }

        fn dispose(&self) {
            self.overlay.unparent();
        }
    }

    impl WidgetImpl for CircularProgressBar {}
    impl DrawingAreaImpl for CircularProgressBar {}
}

glib::wrapper! {
    pub(crate) struct CircularProgressBar(ObjectSubclass<imp::CircularProgressBar>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for CircularProgressBar {
    fn default() -> Self {
        glib::Object::builder::<Self>().build()
    }
}

impl CircularProgressBar {
    pub(crate) fn percentage(&self) -> f64 {
        self.imp().percentage.get()
    }

    pub(crate) fn set_percentage(&self, value: f64) {
        if self.percentage() == value {
            return;
        }

        let imp = self.imp();

        imp.percentage.set(value);
        imp.drawing_area.queue_draw();
        self.notify("percentage");
    }

    pub(crate) fn icon_name(&self) -> Option<glib::GString> {
        self.imp().icon.icon_name()
    }

    pub(crate) fn set_icon_name(&self, value: Option<&str>) {
        self.imp().icon.set_icon_name(value);
    }
}
