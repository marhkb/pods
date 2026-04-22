use std::cell::Cell;
use std::cell::RefCell;
use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use glib::subclass::Signal;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::utils;

const ACTION_SCROLL_DOWN: &str = "auto-scroll-window.scroll-down";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::AutoScrolledWindow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/auto_scrolled_window.ui")]
    pub(crate) struct AutoScrolledWindow {
        pub(super) is_auto_scrolling: Cell<bool>,
        pub(super) prev_adj_upper: Cell<f64>,
        pub(super) prev_adj_value: Cell<f64>,

        #[property(get, set, nullable)]
        pub(super) child: RefCell<Option<gtk::Widget>>,
        #[property(get, set)]
        pub(super) sticky: Cell<bool>,

        #[template_child]
        pub(super) scrolled_window: TemplateChild<gtk::ScrolledWindow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AutoScrolledWindow {
        const NAME: &'static str = "PdsAutoScrolledWindow";
        type Type = super::AutoScrolledWindow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_SCROLL_DOWN, None, |widget, _, _| {
                widget.scroll_down();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AutoScrolledWindow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("scrolled-up").build()])
        }

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

            let adj = obj.vadjustment();
            self.on_adjustment_value_changed(&adj);
            adj.connect_value_changed(clone!(
                #[weak]
                obj,
                move |adj| {
                    obj.imp().on_adjustment_value_changed(adj);
                }
            ));
            adj.connect_upper_notify(clone!(
                #[weak]
                obj,
                move |adj| {
                    let imp = obj.imp();

                    if adj.upper() != imp.prev_adj_upper.get() && obj.sticky() {
                        obj.scroll_down();
                    }

                    imp.prev_adj_upper.set(adj.upper());
                }
            ));
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for AutoScrolledWindow {}

    impl AutoScrolledWindow {
        fn on_adjustment_value_changed(&self, adj: &gtk::Adjustment) {
            let obj = &*self.obj();

            if self.is_auto_scrolling.get() {
                if adj.value() + adj.page_size() >= adj.upper() {
                    self.is_auto_scrolling.set(false);
                    obj.set_sticky(true);
                }
            } else {
                obj.set_sticky(adj.value() + adj.page_size() >= adj.upper());
                if adj.value() < self.prev_adj_value.get() && adj.value() < adj.page_size() {
                    obj.emit_by_name::<()>("scrolled-up", &[]);
                }
            }

            self.prev_adj_value.set(adj.value());
        }
    }
}

glib::wrapper! {
    pub(crate) struct AutoScrolledWindow(ObjectSubclass<imp::AutoScrolledWindow>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl AutoScrolledWindow {
    pub(crate) fn scroll_down(&self) {
        let imp = self.imp();

        imp.is_auto_scrolling.set(true);
        glib::idle_add_local_once(clone!(
            #[weak(rename_to = obj)]
            self,
            move || {
                obj.imp().scrolled_window.vadjustment().set_value(f64::MAX);
            }
        ));
    }

    pub(crate) fn vadjustment(&self) -> gtk::Adjustment {
        self.imp().scrolled_window.vadjustment()
    }
}
