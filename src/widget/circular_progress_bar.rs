use std::cell::Cell;
use std::f64;
use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::graphene;
use gtk::gsk;
use gtk::prelude::ParamSpecBuilderExt;
use gtk::CompositeTemplate;

use crate::utils;

const SIZE: i32 = 32;
const BORDER_WIDTH: i32 = 6;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/circular_progress_bar.ui")]
    pub(crate) struct CircularProgressBar {
        pub(super) percentage: Cell<f64>,
        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CircularProgressBar {
        const NAME: &'static str = "PdsCircularProgressBar";
        type Type = super::CircularProgressBar;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CircularProgressBar {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecDouble::builder("percentage")
                        .maximum(1.0)
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecString::builder("icon-name")
                        .explicit_notify()
                        .build(),
                ]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "percentage" => obj.set_percentage(value.get().unwrap()),
                "icon-name" => obj.set_icon_name(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "percentage" => obj.percentage().to_value(),
                "icon-name" => obj.icon_name().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let adw_style_manager = adw::StyleManager::default();
            adw_style_manager
                .connect_high_contrast_notify(clone!(@weak obj => move |_| obj.queue_draw()));
            adw_style_manager.connect_dark_notify(clone!(@weak obj => move |_| obj.queue_draw()));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for CircularProgressBar {
        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            self.image.measure(orientation, for_size);
            (SIZE, SIZE, -1, -1)
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
            self.image.size_allocate(
                &gtk::Allocation::new(
                    BORDER_WIDTH,
                    BORDER_WIDTH,
                    width - (BORDER_WIDTH * 2),
                    height - (BORDER_WIDTH * 2),
                ),
                baseline,
            );
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = &*self.obj();

            let style_manager = adw::StyleManager::default();
            let style_context = widget.style_context();

            let mut percentage = widget.percentage() as f32;
            if percentage < 0.005 {
                percentage = 0.0;
            }

            let fg_color = if percentage < 0.8 {
                style_context
                    .lookup_color("accent_color")
                    .unwrap_or_else(|| {
                        if style_manager.is_dark() {
                            gdk::RGBA::new(0.471, 0.682, 0.929, 1.0)
                        } else {
                            gdk::RGBA::new(0.11, 0.443, 0.847, 1.0)
                        }
                    })
            } else if percentage < 0.95 {
                style_context
                    .lookup_color("warning_color")
                    .unwrap_or_else(|| {
                        if style_manager.is_dark() {
                            gdk::RGBA::new(0.973, 0.894, 0.361, 1.0)
                        } else {
                            gdk::RGBA::new(0.612, 0.431, 0.012, 1.0)
                        }
                    })
            } else {
                style_context
                    .lookup_color("error_color")
                    .unwrap_or_else(|| {
                        if style_manager.is_dark() {
                            gdk::RGBA::new(1.0, 0.482, 0.388, 1.0)
                        } else {
                            gdk::RGBA::new(0.753, 0.11, 0.157, 1.0)
                        }
                    })
            };

            let bg_color = if style_manager.is_high_contrast() {
                style_context
                    .lookup_color("dark_1")
                    .unwrap_or_else(|| gdk::RGBA::new(0.467, 0.463, 0.482, 1.0))
            } else {
                style_context
                    .lookup_color("window_fg_color")
                    .map(|color| {
                        gdk::RGBA::new(
                            color.red(),
                            color.green(),
                            color.blue(),
                            if style_manager.is_dark() {
                                0.15
                            } else {
                                // FIXME: Find the reason why we need 0.12 to match colors of
                                // container-status-* which have 'alpha(@window_fg_color, .15)'.
                                0.12
                            },
                        )
                    })
                    .unwrap_or_else(|| {
                        if style_manager.is_dark() {
                            gdk::RGBA::new(1.0, 1.0, 1.0, 0.15)
                        } else {
                            gdk::RGBA::new(0.0, 0.0, 0.0, 0.12)
                        }
                    })
            };

            let size_outer = SIZE as f32;
            let rect_outer = graphene::Rect::new(0.0, 0.0, size_outer, size_outer);
            let border_width = BORDER_WIDTH as f32;
            let size_inner = size_outer - border_width;
            let rect_inner = graphene::Rect::new(
                border_width / 2.0,
                border_width / 2.0,
                size_inner,
                size_inner,
            );

            let child_snapshot = gtk::Snapshot::new();
            child_snapshot
                .push_rounded_clip(&gsk::RoundedRect::from_rect(rect_outer, size_outer / 2.0));
            child_snapshot.append_conic_gradient(
                &rect_outer,
                &graphene::Point::new(size_outer / 2.0, size_outer / 2.0),
                0.0,
                &[
                    gsk::ColorStop::new(percentage, fg_color),
                    gsk::ColorStop::new(percentage, gdk::RGBA::new(0.0, 0.0, 0.0, 0.0)),
                ],
            );
            child_snapshot.pop();

            snapshot.push_rounded_clip(&if style_manager.is_high_contrast() {
                gsk::RoundedRect::from_rect(rect_inner, size_inner / 2.0)
            } else {
                gsk::RoundedRect::from_rect(rect_outer, size_outer / 2.0)
            });
            snapshot.append_color(&bg_color, &rect_outer);
            snapshot.pop();

            snapshot.push_mask(gsk::MaskMode::InvertedAlpha);

            snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(rect_inner, size_inner / 2.0));
            snapshot.append_color(&gdk::RGBA::GREEN, &rect_inner);
            snapshot.pop();
            snapshot.pop();

            snapshot.append_node(child_snapshot.to_node().unwrap());
            snapshot.pop();

            widget.snapshot_child(&*self.image, snapshot);
        }
    }

    #[gtk::template_callbacks]
    impl CircularProgressBar {
        #[template_callback]
        fn on_image_notify_icon_name(&self) {
            self.obj().notify("icon-name");
        }
    }
}

glib::wrapper! {
    pub(crate) struct CircularProgressBar(ObjectSubclass<imp::CircularProgressBar>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for CircularProgressBar {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl CircularProgressBar {
    pub(crate) fn percentage(&self) -> f64 {
        self.imp().percentage.get()
    }

    pub(crate) fn set_percentage(&self, value: f64) {
        let value = (value.clamp(0.0, 1.0) * 100.0).round() / 100.0;

        if self.percentage() != value {
            self.imp().percentage.set(value);
            self.queue_draw();
            self.notify("percentage");
        }
    }

    pub(crate) fn icon_name(&self) -> Option<glib::GString> {
        self.imp().image.icon_name()
    }

    pub(crate) fn set_icon_name(&self, value: Option<&str>) {
        self.imp().image.set_icon_name(value);
    }
}
