use std::cell::Cell;
use std::cell::OnceCell;
use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::graphene;
use gtk::gsk;

use crate::utils;

const SIZE: i32 = 34;
const BORDER_WIDTH: i32 = 6;
const BORDER_MARGIN: i32 = 1;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/spinner.ui")]
    pub(crate) struct Spinner {
        pub(super) spinning: Cell<bool>,
        pub(super) animation: OnceCell<adw::TimedAnimation>,
        pub(super) animation_value: Cell<f32>,
        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Spinner {
        const NAME: &'static str = "PdsSpinner";
        type Type = super::Spinner;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Spinner {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecString::builder("icon-name")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecBoolean::builder("spinning")
                        .explicit_notify()
                        .build(),
                ]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "icon-name" => obj.set_icon_name(value.get().unwrap_or_default()),
                "spinning" => obj.set_spinning(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "icon-name" => obj.icon_name().to_value(),
                "spinning" => obj.is_spinning().to_value(),
                _ => unimplemented!(),
            }
        }
        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let target = adw::CallbackAnimationTarget::new(clone!(
                #[weak]
                obj,
                move |value| {
                    obj.imp().animation_value.set(value as f32);
                    obj.queue_draw();
                }
            ));
            let animation = adw::TimedAnimation::builder()
                .widget(obj)
                .target(&target)
                .value_from(0.0)
                .value_to(6.0)
                .duration(3200)
                .repeat_count(0)
                .easing(adw::Easing::Linear)
                .build();
            self.animation.set(animation).unwrap();

            let adw_style_manager = adw::StyleManager::default();
            adw_style_manager.connect_high_contrast_notify(clone!(
                #[weak]
                obj,
                move |_| obj.queue_draw()
            ));
            adw_style_manager.connect_dark_notify(clone!(
                #[weak]
                obj,
                move |_| obj.queue_draw()
            ));
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for Spinner {
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
                    width - BORDER_WIDTH * 2,
                    height - BORDER_WIDTH * 2,
                ),
                baseline,
            );
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = &*self.obj();

            let style_context = widget.style_context();

            let animation_value: f32 = self.animation_value.get();

            let border_margin = BORDER_MARGIN as f32;

            let outer_clip_size = SIZE as f32 - border_margin * 2.0;
            let outer_clip_rect = graphene::Rect::new(
                border_margin,
                border_margin,
                outer_clip_size,
                outer_clip_size,
            );

            let child_snapshot = gtk::Snapshot::new();

            child_snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(
                outer_clip_rect,
                outer_clip_size / 2.0,
            ));

            if widget.is_spinning() {
                let is_growing = animation_value as i32 % 2 == 0;

                let percentage = if is_growing {
                    animation_value % 1.0
                } else {
                    1.0 - animation_value % 1.0
                };

                let percentage_clamped = percentage.clamp(0.15, 0.75);
                child_snapshot.append_conic_gradient(
                    &outer_clip_rect,
                    &graphene::Point::new(outer_clip_size / 2.0, outer_clip_size / 2.0),
                    if is_growing {
                        percentage * 60.0
                    } else {
                        (2.0 - percentage) * 60.0 + (1.0 - percentage) * 360.0
                    } + 120.0 * (animation_value / 2.0).floor()
                        + (percentage - percentage_clamped) * 180.0,
                    &[
                        gsk::ColorStop::new(percentage_clamped, style_context.color()),
                        gsk::ColorStop::new(percentage_clamped, gdk::RGBA::TRANSPARENT),
                    ],
                );
            } else {
                child_snapshot.append_color(&gdk::RGBA::TRANSPARENT, &outer_clip_rect);
            }

            child_snapshot.pop();

            snapshot.push_mask(gsk::MaskMode::InvertedAlpha);

            let border_width = BORDER_WIDTH as f32;

            let inner_clip_anchor = border_width / 2.0 + border_margin;
            let inner_clip_size = outer_clip_size - border_width;
            let inner_clip_rect = graphene::Rect::new(
                inner_clip_anchor,
                inner_clip_anchor,
                inner_clip_size,
                inner_clip_size,
            );

            snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(
                inner_clip_rect,
                inner_clip_size / 2.0,
            ));
            snapshot.append_color(&gdk::RGBA::GREEN, &inner_clip_rect);
            snapshot.pop();
            snapshot.pop();

            snapshot.append_node(child_snapshot.to_node().unwrap());
            snapshot.pop();

            widget.snapshot_child(&*self.image, snapshot);
        }

        fn map(&self) {
            self.parent_map();

            if self.obj().is_spinning() {
                self.animation.get().unwrap().play();
            }
        }
    }

    #[gtk::template_callbacks]
    impl Spinner {
        #[template_callback]
        fn on_image_notify_icon_name(&self) {
            let obj = &*self.obj();
            obj.queue_draw();
            obj.notify("icon-name");
        }
    }
}

glib::wrapper! {
    pub(crate) struct Spinner(ObjectSubclass<imp::Spinner>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Spinner {
    pub(crate) fn icon_name(&self) -> Option<glib::GString> {
        self.imp().image.icon_name()
    }

    pub(crate) fn set_icon_name(&self, value: Option<&str>) {
        self.imp().image.set_icon_name(value);
    }

    pub(crate) fn is_spinning(&self) -> bool {
        self.imp().spinning.get()
    }

    pub(crate) fn set_spinning(&self, value: bool) {
        if self.is_spinning() == value {
            return;
        }

        let imp = self.imp();

        let animation = imp.animation.get().unwrap();
        if value {
            animation.play();
        } else {
            animation.pause();
            imp.animation_value.set(0.0);
        }

        imp.spinning.set(value);
        self.queue_draw();
        self.notify("spinning");
    }
}
