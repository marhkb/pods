use std::cell::Cell;

use adw::traits::BinExt;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::gsk;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/component/count-badge.ui")]
    pub(crate) struct CountBadge {
        pub(super) count: Cell<u32>,
        pub(super) mask_shader: OnceCell<Option<gsk::GLShader>>,
        #[template_child]
        pub(super) child_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) count_mask: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) count_badge: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) count_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CountBadge {
        const NAME: &'static str = "PdsCountBadge";
        type Type = super::CountBadge;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.set_css_name("countbadge");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CountBadge {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "child",
                        "Child",
                        "The count to display in the badge",
                        gtk::Widget::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecUInt::new(
                        "count",
                        "Count",
                        "The count to display in the badge",
                        0,
                        u32::MAX,
                        0,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.instance();
            match pspec.name() {
                "child" => obj.set_child(value.get().unwrap_or_default()),
                "count" => obj.set_count(value.get().unwrap_or_default()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.instance();
            match pspec.name() {
                "child" => obj.child().to_value(),
                "count" => obj.count().to_value(),
                _ => unimplemented!(),
            }
        }
        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            self.child_bin
                .connect_child_notify(clone!(@weak obj => move |_| {
                    obj.notify("child");
                }));

            Self::Type::this_expression("count")
                .chain_closure::<String>(closure!(|_: Self::Type, count: u32| {
                    if count <= 9 {
                        count.to_string()
                    } else {
                        "+".to_owned()
                    }
                }))
                .bind(&*self.count_label, "label", Some(obj));

            Self::Type::this_expression("count").watch(
                Some(obj),
                clone!(@weak obj => move || {
                    obj.queue_draw();
                }),
            );
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.instance()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for CountBadge {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = &*self.instance();

            if widget.count() == 0 {
                widget.snapshot_child(&*self.child_bin, snapshot);
                return;
            }

            let child_snapshot = gtk::Snapshot::new();
            widget.snapshot_child(&*self.child_bin, &child_snapshot);

            if let Some(child_node) = child_snapshot.to_node() {
                widget.ensure_mask_shader();

                let maybe_compiled_masked_shader = self.mask_shader.get().unwrap();

                if let Some(ref compiled_mask_shader) = maybe_compiled_masked_shader {
                    snapshot.push_gl_shader(
                        compiled_mask_shader,
                        &child_node.bounds(),
                        &gsk::ShaderArgsBuilder::new(compiled_mask_shader, None).to_args(),
                    );
                }

                snapshot.append_node(&child_node);

                if maybe_compiled_masked_shader.is_some() {
                    snapshot.gl_shader_pop_texture();
                    widget.snapshot_child(&*self.count_mask, snapshot);
                    snapshot.gl_shader_pop_texture();

                    snapshot.pop();
                }
            } else {
                widget.snapshot_child(&*self.count_mask, snapshot);
            }

            widget.snapshot_child(&*self.count_badge, snapshot);
        }
    }
}

glib::wrapper! {
    pub(crate) struct CountBadge(ObjectSubclass<imp::CountBadge>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl CountBadge {
    pub(crate) fn child(&self) -> Option<gtk::Widget> {
        self.imp().child_bin.child()
    }

    pub(crate) fn set_child(&self, value: Option<&gtk::Widget>) {
        if self.child().as_ref() == value {
            return;
        }
        self.imp().child_bin.set_child(value);
    }

    pub(crate) fn count(&self) -> u32 {
        self.imp().count.get()
    }

    pub(crate) fn set_count(&self, value: u32) {
        if self.count() == value {
            return;
        }
        self.imp().count.set(value);
        self.notify("count");
    }

    fn ensure_mask_shader(&self) {
        let imp = self.imp();

        if imp.mask_shader.get().is_some() {
            // We've already tried to compile the shader before.
            return;
        }

        let shader = gsk::GLShader::from_resource("/org/gnome/Adwaita/glsl/mask.glsl");
        let renderer = self.native().unwrap().renderer();
        let compiled_shader = match shader.compile(&renderer) {
            Err(e) => {
                // If shaders aren't supported, the error doesn't matter and we just silently fall
                // back.
                log::error!("Couldn't compile shader: {}", e);
                None
            }
            Ok(_) => Some(shader),
        };

        imp.mask_shader.set(compiled_shader).unwrap();
    }
}
