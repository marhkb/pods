use std::cell::{Cell, RefCell};
use std::mem;

use futures::stream;
use gtk::glib::{clone, WeakRef};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use once_cell::sync::Lazy;

use crate::{model, utils};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container-logs-panel.ui")]
    pub(crate) struct ContainerLogsPanel {
        pub(super) container: WeakRef<model::Container>,
        pub(super) abort_handle: RefCell<Option<stream::AbortHandle>>,
        pub(super) handler_id: RefCell<Option<glib::SignalHandlerId>>,
        pub(super) last_log_time: RefCell<Option<glib::DateTime>>,
        pub(super) is_auto_scrolling: Cell<bool>,
        pub(super) sticky: Cell<bool>,
        #[template_child]
        pub(super) overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub(super) text_buffer: TemplateChild<gtk::TextBuffer>,
        #[template_child]
        pub(super) text_view: TemplateChild<gtk::TextView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerLogsPanel {
        const NAME: &'static str = "ContainerLogsPanel";
        type Type = super::ContainerLogsPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("logs.scroll-down", None, move |widget, _, _| {
                widget.scroll_down();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerLogsPanel {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "container",
                        "Container",
                        "The container of this ContainerLogsPanel",
                        model::Container::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecBoolean::new(
                        "sticky",
                        "Sticky",
                        "Whether the log should stick to the last message",
                        true,
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
                "container" => obj.set_container(value.get().unwrap()),
                "sticky" => obj.set_sticky(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container" => obj.container().to_value(),
                "sticky" => obj.sticky().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            self.text_view.set_property("monospace", true);
            self.text_view.set_property("cursor-visible", false);
            let adj = self.scrolled_window.vadjustment();
            adj.connect_value_changed(clone!(@weak obj => move |adj| {
                let self_ = Self::from_instance(&obj);

                if self_.is_auto_scrolling.get() {
                    if adj.value() + adj.page_size() >= adj.upper() {
                        self_.is_auto_scrolling.set(false);
                        obj.set_sticky(true);
                    }
                } else {
                    obj.set_sticky(adj.value() + adj.page_size() >= adj.upper());
                }
            }));

            adj.connect_upper_notify(clone!(@weak obj => move |_| {
                if obj.sticky() || Self::from_instance(&obj).is_auto_scrolling.get() {
                    obj.scroll_down();
                }
            }));

            obj.scroll_down();
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.overlay.unparent();
        }
    }

    impl WidgetImpl for ContainerLogsPanel {}
}

glib::wrapper! {
    pub(crate) struct ContainerLogsPanel(ObjectSubclass<imp::ContainerLogsPanel>)
        @extends gtk::Widget;
}

impl ContainerLogsPanel {
    fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    fn set_container(&self, value: Option<&model::Container>) {
        let container = self.container();

        if container.as_ref() == value {
            return;
        }

        self.abort();

        let imp = self.imp();

        if let Some(container) = container {
            container.disconnect(imp.handler_id.take().unwrap());
        }

        if let Some(container) = value {
            self.connect_logs(container, None);

            let handler_id = container.connect_notify_local(
                Some("status"),
                clone!(@weak self as obj => move |container, _| {
                    if let model::ContainerStatus::Running = container.status() {
                        obj.abort();
                        obj.connect_logs(container, obj.imp().last_log_time.borrow().clone());
                    }
                }),
            );
            imp.handler_id.replace(Some(handler_id));
        }

        imp.container.set(value);
        self.notify("container");
    }

    fn sticky(&self) -> bool {
        self.imp().sticky.get()
    }

    fn set_sticky(&self, sticky: bool) {
        if self.sticky() == sticky {
            return;
        }

        self.imp().sticky.set(sticky);
        self.notify("sticky");
    }

    fn scroll_down(&self) {
        let imp = self.imp();

        imp.is_auto_scrolling.set(true);
        imp.scrolled_window
            .emit_by_name::<bool>("scroll-child", &[&gtk::ScrollType::End, &false]);
    }

    fn connect_logs(&self, container: &model::Container, since: Option<glib::DateTime>) {
        let mut perform = MarkupPerform::default();

        utils::run_stream(
            container.logs(since),
            clone!(@weak self as obj => @default-return glib::Continue(false), move |result| {
                glib::Continue(match result {
                    Ok(line) => {
                        let imp = obj.imp();
                        if line.len() > 8 && perform.decode(&line[8..]) {

                            imp.text_buffer.insert_markup(
                                &mut imp.text_buffer.end_iter(),
                                &format!("{}\n", perform.move_out_buffer()),
                            );
                        }
                        imp.last_log_time.replace(glib::DateTime::now_local().ok());
                        true
                    }
                    Err(e) => {
                        log::warn!("Stopping container log stream due to error: {e}");
                        false
                    }
                })
            }),
        );
    }

    fn abort(&self) {
        if let Some(handle) = self.imp().abort_handle.take() {
            handle.abort();
        }
    }
}

#[derive(Debug, Default)]
pub struct MarkupPerform {
    buffer: String,
    close_tags: Vec<&'static str>,
}

impl MarkupPerform {
    fn move_out_buffer(&mut self) -> String {
        let mut buffer = String::new();
        mem::swap(&mut self.buffer, &mut buffer);
        buffer
    }

    /// Decode the specified bytes. Return true if finished.
    fn decode(&mut self, ansi_encoded_bytes: &[u8]) -> bool {
        let mut parser = vte::Parser::new();

        String::from_utf8_lossy(ansi_encoded_bytes)
            .bytes()
            .for_each(|byte| parser.advance(self, byte));

        self.close_tags.is_empty()
    }
}

impl vte::Perform for MarkupPerform {
    fn print(&mut self, c: char) {
        self.buffer.push(c);
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
        for param in params.iter() {
            match param {
                [0] => {
                    while let Some(close_tag) = self.close_tags.pop() {
                        self.buffer.push_str(close_tag);
                    }
                }
                items => items
                    .iter()
                    .copied()
                    .filter_map(ansi_escape_to_markup_tags)
                    .for_each(|(start_tag, close_tag)| {
                        self.buffer.push_str(start_tag);
                        self.close_tags.push(close_tag);
                    }),
            }
        }
    }
}

fn ansi_escape_to_markup_tags(item: u16) -> Option<(&'static str, &'static str)> {
    Some(match item {
        1 => ("<b>", "</b>"),
        30 => ("<span foreground=\"#000000\">", "</span>"),
        31 => ("<span foreground=\"#e01b24\">", "</span>"),
        32 => ("<span foreground=\"#33d17a\">", "</span>"),
        33 => ("<span foreground=\"#f6d32d\">", "</span>"),
        34 => ("<span foreground=\"#3584e4\">", "</span>"),
        35 => ("<span foreground=\"#d4267e\">", "</span>"),
        36 => ("<span foreground=\"#00f7f7\">", "</span>"),
        37 => ("<span foreground=\"#ffffff\">", "</span>"),
        _ => return None,
    })
}
