// Inspired by https://github.com/phastmike/vala-circular-progress-bar/blob/1528d42a6045734038bf0022a88b846edf582b3a/circular-progress-bar.vala.

use std::cell::Cell;
use std::cell::RefCell;
use std::f64;
use std::time::Duration;

use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/circular-progress-bar.ui")]
    pub(crate) struct CircularProgressBar {
        pub(super) percentage: Cell<f64>,
        pub(super) current_percentage: Cell<f64>,
        pub(super) signum: Cell<f64>,
        pub(super) source: RefCell<Option<glib::SourceId>>,
        #[template_child]
        pub(super) overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) description_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) percentage_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) drawing_area: TemplateChild<gtk::DrawingArea>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CircularProgressBar {
        const NAME: &'static str = "CircularProgressBar";
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
                    glib::ParamSpecDouble::new(
                        "percentage",
                        "Percentage",
                        "The progress in percentage",
                        0.0,
                        1.0,
                        0.0,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecString::new(
                        "label",
                        "Label",
                        "The label that will be displayed within the circle",
                        None,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
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
                "percentage" => obj.set_percentage(value.get().unwrap()),
                "label" => obj.set_label(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "percentage" => obj.percentage().to_value(),
                "label" => obj.label().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            self.description_label.connect_notify_local(
                Some("label"),
                clone!(@weak obj => move |_, _| obj.notify("label")),
            );

            self.drawing_area
                .set_draw_func(clone!(@weak obj => move |_, cr, w, h| {
                    let colors = if adw::StyleManager::default().is_dark() {
                        [
                            // @dark_2
                            (0.368, 0.360, 0.392),
                            // @accent_color
                            (0.470, 0.682, 0.929),
                            // @warning_color
                            (0.972, 0.894, 0.360),
                            // @error_color
                            (1.0, 0.482, 0.388)
                        ]
                    } else {
                        [
                            // @light_3
                            (0.870, 0.866, 0.854),
                            // @accent_color
                            (0.109, 0.443, 0.847),
                            // @warning_color
                            (0.682, 0.482, 0.011),
                            // @error_color
                            (0.752, 0.109, 0.156)
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
                    cr.set_line_width(line_width_fill);
                    cr.arc(center_x, center_y, delta_fill, 0.0, 2. * pi);
                    cr.set_source_rgb(colors[0].0, colors[0].1, colors[0].2);
                    cr.stroke().unwrap();

                    // Percentage
                    let line_width_percentage = 3.0;
                    let delta_percentage = radius - (line_width_percentage / 2.0);

                    let current_percentage = obj.current_percentage();
                    if current_percentage < 0.8 {
                        cr.set_source_rgb(colors[1].0, colors[1].1, colors[1].2);
                    } else if current_percentage < 0.95 {
                        cr.set_source_rgb(colors[2].0, colors[2].1, colors[2].2);
                    } else {
                        cr.set_source_rgb(colors[3].0, colors[3].1, colors[3].2);
                    }

                    cr.set_line_width(line_width_percentage);
                    cr.arc(
                        center_x,
                        center_y,
                        delta_percentage,
                        1.5 * pi,
                        (1.5 + current_percentage * 2.0) * pi,
                    );
                    cr.stroke().unwrap();

                    cr.restore().unwrap();
                }));

            adw::StyleManager::default().connect_dark_notify(clone!(@weak obj => move |_| {
                obj.imp().drawing_area.queue_draw();
            }));

            Self::Type::this_expression("percentage")
                .chain_closure::<String>(closure!(|_: glib::Object, percentage: f64| {
                    let percentage = percentage * 100.0;
                    if percentage < 10.0 {
                        format!("{:.1}%", percentage)
                    } else {
                        format!("{:.0}%", percentage)
                    }
                }))
                .bind(&*self.percentage_label, "label", Some(obj));
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.overlay.unparent();
        }
    }

    impl WidgetImpl for CircularProgressBar {}
    impl DrawingAreaImpl for CircularProgressBar {}
}

glib::wrapper! {
    pub(crate) struct CircularProgressBar(ObjectSubclass<imp::CircularProgressBar>)
        @extends gtk::Widget;
}

impl Default for CircularProgressBar {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create CircularProgressBar")
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

        if let Some(source) = imp.source.take() {
            source.remove();
        }

        let diff = value - imp.percentage.get();
        imp.signum.set(diff.signum());
        imp.percentage.set(value);

        let step = diff.abs() * 0.03 + 0.001;
        let source = glib::timeout_add_local(
            Duration::from_millis((500.0 / (diff.abs() / step)) as u64),
            clone!(@weak self as obj => @default-return glib::Continue(false), move || {
                let imp = obj.imp();

                imp.drawing_area.queue_draw();

                let percentage = obj.percentage();

                let current = obj.current_percentage();
                let signum = imp.signum.get();

                let current_next = current + step * signum;

                glib::Continue(
                    if (signum > 0.0 && current_next >= percentage)
                        || (signum < 0.0 && current_next <= percentage)
                    {
                        obj.set_current_percentage(percentage);
                        if let Some(source) = imp.source.take() {
                            source.remove();
                        }
                        false
                    } else {
                        obj.set_current_percentage(current_next);
                        true
                    },
                )
            }),
        );
        imp.source.replace(Some(source));

        self.notify("percentage");
    }

    fn current_percentage(&self) -> f64 {
        self.imp().current_percentage.get()
    }

    fn set_current_percentage(&self, value: f64) {
        if self.current_percentage() == value {
            return;
        }

        let imp = self.imp();

        if value < 0.95 {
            imp.overlay.remove_css_class("error");
        } else {
            imp.overlay.add_css_class("error");
        }

        imp.current_percentage.set(value);
    }

    pub(crate) fn label(&self) -> glib::GString {
        self.imp().description_label.label()
    }

    pub(crate) fn set_label(&self, value: &str) {
        if self.label().as_str() == value {
            return;
        }
        self.imp().description_label.set_label(value);
        self.notify("label");
    }
}
