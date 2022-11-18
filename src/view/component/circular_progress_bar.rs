use std::cell::Cell;
use std::f64;

use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::graphene;
use gtk::gsk;
use gtk::prelude::ParamSpecBuilderExt;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::utils;

const SIZE: i32 = 32;
const BORDER_WITH: f32 = 6.0;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/component/circular-progress-bar.ui")]
    pub(crate) struct CircularProgressBar {
        pub(super) percentage: Cell<f64>,
        #[template_child]
        pub(super) icon: TemplateChild<gtk::Image>,
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

            self.icon.connect_notify_local(
                Some("icon-name"),
                clone!(@weak obj => move |_, _| obj.notify("icon-name")),
            );
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for CircularProgressBar {
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
                            gdk::RGBA::new(0.470, 0.682, 0.929, 1.0)
                        } else {
                            gdk::RGBA::new(0.109, 0.443, 0.847, 1.0)
                        }
                    })
            } else if percentage < 0.95 {
                style_context
                    .lookup_color("warning_color")
                    .unwrap_or_else(|| {
                        if style_manager.is_dark() {
                            gdk::RGBA::new(0.972, 0.894, 0.360, 1.0)
                        } else {
                            gdk::RGBA::new(0.682, 0.482, 0.011, 1.0)
                        }
                    })
            } else {
                style_context
                    .lookup_color("error_color")
                    .unwrap_or_else(|| {
                        if style_manager.is_dark() {
                            gdk::RGBA::new(1.0, 0.482, 0.388, 1.0)
                        } else {
                            gdk::RGBA::new(0.752, 0.109, 0.156, 1.0)
                        }
                    })
            };
            let bg_color = if style_manager.is_dark() {
                style_context
                    .lookup_color("dark_2")
                    .unwrap_or_else(|| gdk::RGBA::new(0.369, 0.361, 0.392, 1.0))
            } else {
                style_context
                    .lookup_color("light_3")
                    .unwrap_or_else(|| gdk::RGBA::new(0.753, 0.749, 0.737, 1.0))
            };

            let size = SIZE as f32;
            let rect = graphene::Rect::new(2.0, 2.0, size, size);
            snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(rect, size / 2.0));
            snapshot.append_conic_gradient(
                &rect,
                &graphene::Point::new(size / 2.0, size / 2.0),
                0.0,
                &[
                    gsk::ColorStop::new(percentage, fg_color),
                    gsk::ColorStop::new(percentage, bg_color),
                ],
            );
            snapshot.pop();

            let size = size - BORDER_WITH;
            let rect =
                graphene::Rect::new(BORDER_WITH / 2.0 + 2.0, BORDER_WITH / 2.0 + 2.0, size, size);
            snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(rect, size / 2.0));
            snapshot.append_color(&bg_color, &rect);
            snapshot.pop();

            widget.snapshot_child(&*self.icon, snapshot);
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

        self.imp().percentage.set(value.clamp(0.0, 1.0));
        self.queue_draw();
        self.notify("percentage");
    }

    pub(crate) fn icon_name(&self) -> Option<glib::GString> {
        self.imp().icon.icon_name()
    }

    pub(crate) fn set_icon_name(&self, value: Option<&str>) {
        self.imp().icon.set_icon_name(value);
    }
}
