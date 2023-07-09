use std::cell::Cell;
use std::sync::OnceLock;

use adw::traits::BinExt;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::gsk;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::CountBadge)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/count_badge.ui")]
    pub(crate) struct CountBadge {
        #[property(get, set = Self::set_count, explicit_notify)]
        pub(super) count: Cell<u32>,
        #[template_child]
        pub(super) child_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) count_mask: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) count_badge: TemplateChild<gtk::Widget>,
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
            Self::bind_template_callbacks(klass);
            klass.set_css_name("countbadge");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CountBadge {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                Self::derived_properties()
                    .iter()
                    .cloned()
                    .chain(Some(
                        glib::ParamSpecObject::builder::<gtk::Widget>("child")
                            .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                            .build(),
                    ))
                    .collect::<Vec<_>>()
            })
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = &*self.obj();
            match pspec.name() {
                "child" => obj.set_child(value.get().unwrap_or_default()),
                _ => self.derived_set_property(id, value, pspec),
            }
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "child" => obj.child().to_value(),
                _ => self.derived_property(id, pspec),
            }
        }
        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            Self::Type::this_expression("count")
                .chain_closure::<String>(closure!(|_: Self::Type, count: u32| {
                    if count <= 9 {
                        count.to_string()
                    } else {
                        "+".to_owned()
                    }
                }))
                .bind(&*self.count_label, "label", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref())
        }
    }

    impl WidgetImpl for CountBadge {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = &*self.obj();

            if widget.count() == 0 {
                widget.snapshot_child(&*self.child_bin, snapshot);
                return;
            }

            let child_snapshot = gtk::Snapshot::new();
            widget.snapshot_child(&*self.child_bin, &child_snapshot);

            snapshot.push_mask(gsk::MaskMode::InvertedAlpha);

            widget.snapshot_child(&*self.count_mask, snapshot);
            snapshot.pop();

            snapshot.append_node(&child_snapshot.to_node().unwrap());
            snapshot.pop();

            widget.snapshot_child(&*self.count_badge, snapshot);
        }
    }

    #[gtk::template_callbacks]
    impl CountBadge {
        #[template_callback]
        fn on_child_bin_notify_child(&self) {
            self.obj().notify("child");
        }

        fn set_count(&self, value: u32) {
            let obj = &*self.obj();

            if obj.count() == value {
                return;
            }

            let needs_redraw = obj.count() == 0 || value == 0;

            self.count.set(value);
            obj.notify_count();

            if needs_redraw {
                obj.queue_draw();
            }
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
}
