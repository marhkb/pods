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
use once_cell::unsync::OnceCell;

use crate::utils;

const SIZE: i32 = 32;
const BORDER_WIDTH: f32 = 6.0;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/component/circular-progress-bar.ui")]
    pub(crate) struct CircularProgressBar {
        pub(super) percentage: Cell<f64>,
        pub(super) mask_shader: OnceCell<Option<gsk::GLShader>>,
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

            let adw_style_manager = adw::StyleManager::default();
            adw_style_manager
                .connect_high_contrast_notify(clone!(@weak obj => move |_| obj.queue_draw()));
            adw_style_manager.connect_dark_notify(clone!(@weak obj => move |_| obj.queue_draw()));
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for CircularProgressBar {
        fn measure(&self, _: gtk::Orientation, _: i32) -> (i32, i32, i32, i32) {
            (SIZE, SIZE, -1, -1)
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
            self.icon
                .size_allocate(&gtk::Allocation::new(-2, 0, width, height), baseline);
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = &*self.obj();

            let style_manager = adw::StyleManager::default();
            let style_context = widget.style_context();

            let mut percentage = widget.percentage() as f32;
            if percentage < 0.005 {
                percentage = 0.0;
            }

            let fg_color = style_context
                .lookup_color(if percentage < 0.8 {
                    "accent_color"
                } else if percentage < 0.95 {
                    "warning_color"
                } else {
                    "error_color"
                })
                .unwrap();

            let (bg_color, maybe_compiled_masked_shader) = if style_manager.is_high_contrast() {
                (style_context.lookup_color("dark_1").unwrap(), None)
            } else {
                let maybe_compiled_masked_shader = widget.ensure_mask_shader();

                let color = if maybe_compiled_masked_shader.is_some() {
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
                        .unwrap()
                } else if style_manager.is_dark() {
                    style_context.lookup_color("dark_2").unwrap()
                } else {
                    style_context.lookup_color("light_3").unwrap()
                };

                (color, maybe_compiled_masked_shader)
            };

            let size_outer = SIZE as f32;
            let rect_outer = graphene::Rect::new(0.0, 0.0, size_outer, size_outer);

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

            if !style_manager.is_high_contrast() {
                snapshot
                    .push_rounded_clip(&gsk::RoundedRect::from_rect(rect_outer, size_outer / 2.0));
                snapshot.append_color(&bg_color, &rect_outer);
                snapshot.pop();
            }

            if let Some(compiled_mask_shader) = maybe_compiled_masked_shader {
                snapshot.push_gl_shader(
                    compiled_mask_shader,
                    &rect_outer,
                    &gsk::ShaderArgsBuilder::new(compiled_mask_shader, None).to_args(),
                );
            }

            snapshot.append_node(&child_snapshot.to_node().unwrap());

            let size_inner = size_outer - BORDER_WIDTH;
            let rect_inner = graphene::Rect::new(
                BORDER_WIDTH / 2.0,
                BORDER_WIDTH / 2.0,
                size_inner,
                size_inner,
            );

            if maybe_compiled_masked_shader.is_some() {
                snapshot.gl_shader_pop_texture();

                snapshot
                    .push_rounded_clip(&gsk::RoundedRect::from_rect(rect_inner, size_inner / 2.0));
                snapshot.append_color(&gdk::RGBA::GREEN, &rect_inner);
                snapshot.pop();

                snapshot.gl_shader_pop_texture();
                snapshot.pop();
            } else {
                snapshot
                    .push_rounded_clip(&gsk::RoundedRect::from_rect(rect_inner, size_inner / 2.0));
                snapshot.append_color(&bg_color, &rect_inner);
                snapshot.pop();
            }

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

    fn ensure_mask_shader(&self) -> Option<&gsk::GLShader> {
        self.imp()
            .mask_shader
            .get_or_init(|| {
                let shader = gsk::GLShader::from_resource("/org/gnome/Adwaita/glsl/mask.glsl");
                let renderer = self.native().unwrap().renderer();

                match shader.compile(&renderer) {
                    Err(e) => {
                        // If shaders aren't supported, the error doesn't matter and we just silently fall
                        // back.
                        log::error!("Couldn't compile shader: {}", e);
                        None
                    }
                    Ok(_) => Some(shader),
                }
            })
            .as_ref()
    }
}
