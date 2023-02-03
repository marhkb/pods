use std::cell::RefCell;

use futures::future;
use futures::AsyncWriteExt;
use futures::StreamExt;
use gettextrs::gettext;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::subclass::Signal;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use vte4::TerminalExt;
use vte4::TerminalExtManual;

use crate::model;
use crate::podman;
use crate::utils;

const ACTION_START_OR_RESUME: &str = "container-tty.start-or-resume";
const ACTION_COPY: &str = "container-tty.copy";
const ACTION_COPY_HTML: &str = "container-tty.copy-html";
const ACTION_PASTE: &str = "container-tty.paste";

#[derive(Debug)]
enum ExecInput {
    Data(Vec<u8>),
    Resize { columns: usize, rows: usize },
}

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container/tty.ui")]
    pub(crate) struct Tty {
        pub(super) settings: utils::PodsSettings,
        pub(super) container: WeakRef<model::Container>,
        pub(super) tx_tokio: RefCell<Option<tokio::sync::mpsc::UnboundedSender<ExecInput>>>,
        #[template_child]
        pub(super) popover_menu: TemplateChild<gtk::PopoverMenu>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) terminal: TemplateChild<vte4::Terminal>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Tty {
        const NAME: &'static str = "PdsContainerTty";
        type Type = super::Tty;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_START_OR_RESUME, None, |widget, _, _| {
                if let Some(container) = widget.container() {
                    if container.can_start() {
                        super::super::start(widget.upcast_ref());
                        widget.action_set_enabled(ACTION_START_OR_RESUME, false);
                    } else if container.can_resume() {
                        super::super::resume(widget.upcast_ref());
                        widget.action_set_enabled(ACTION_START_OR_RESUME, false);
                    }
                }
            });

            klass.install_action(ACTION_COPY, None, |widget, _, _| widget.copy_plain());
            klass.install_action(ACTION_COPY_HTML, None, |widget, _, _| widget.copy_html());
            klass.install_action(ACTION_PASTE, None, |widget, _, _| widget.paste());

            klass.add_binding_action(
                gdk::Key::C,
                gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
                ACTION_COPY,
                None,
            );
            klass.add_binding_action(
                gdk::Key::V,
                gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
                ACTION_PASTE,
                None,
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl Tty {
        #[template_callback]
        fn on_key_pressed(
            &self,
            key: gdk::Key,
            _: u32,
            modifier: gdk::ModifierType,
            _: &gtk::EventControllerKey,
        ) -> gtk::Inhibit {
            glib::signal::Inhibit(if modifier == gdk::ModifierType::CONTROL_MASK {
                if key == gdk::Key::minus || key == gdk::Key::KP_Subtract {
                    self.obj().zoom_out();
                    true
                } else if key == gdk::Key::plus || key == gdk::Key::KP_Add || key == gdk::Key::equal
                {
                    self.obj().zoom_in();
                    true
                } else if key == gdk::Key::_0 {
                    self.obj().zoom_normal();
                    true
                } else {
                    false
                }
            } else if modifier == gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK {
                if key == gdk::Key::C {
                    self.obj().copy_plain();
                    true
                } else if key == gdk::Key::V {
                    self.obj().paste();
                    true
                } else {
                    false
                }
            } else {
                false
            })
        }

        #[template_callback]
        fn on_mouse_pressed(&self, _: i32, x: f64, y: f64) {
            let popover_menu = &*self.popover_menu;
            popover_menu.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 0, 0)));
            popover_menu.popup();
        }

        #[template_callback]
        fn on_scroll(&self, _dx: f64, dy: f64, scroll: gtk::EventControllerScroll) -> gtk::Inhibit {
            gtk::Inhibit(
                if scroll.current_event_state() == gdk::ModifierType::CONTROL_MASK {
                    if dy.is_sign_negative() {
                        self.obj().zoom_in();
                    } else {
                        self.obj().zoom_out();
                    }
                    true
                } else {
                    false
                },
            )
        }
    }

    impl ObjectImpl for Tty {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("terminated").build()]);
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Container>("container")
                        .explicit_notify()
                        .build(),
                    glib::ParamSpecDouble::builder("font-scale")
                        .explicit_notify()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "container" => self.obj().set_container(value.get().unwrap()),
                "font-scale" => self.obj().set_font_scale(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container" => self.obj().container().to_value(),
                "font-scale" => self.obj().font_scale().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            self.terminal.connect_notify_local(
                Some("font-scale"),
                clone!(@weak obj => move |_, _| obj.notify("font-scale")),
            );

            self.settings
                .bind("terminal-font-scale", obj, "font-scale")
                .build();

            self.popover_menu.set_parent(obj);

            obj.update_copy_actions();
            self.terminal
                .connect_selection_changed(clone!(@weak obj => move |_| {
                    obj.update_copy_actions();
                }));

            self.terminal.set_bold_is_bright(true);
            self.terminal.set_colors(
                None,
                None,
                &[
                    &rgba_from_hex(0x17, 0x14, 0x21),
                    &rgba_from_hex(0xc0, 0x1c, 0x28),
                    &rgba_from_hex(0x26, 0xa2, 0x69),
                    &rgba_from_hex(0xa2, 0x73, 0x4c),
                    &rgba_from_hex(0x12, 0x48, 0x8b),
                    &rgba_from_hex(0xa3, 0x47, 0xba),
                    &rgba_from_hex(0x2a, 0xa1, 0xb3),
                    &rgba_from_hex(0xd0, 0xcf, 0xcc),
                    &rgba_from_hex(0x5e, 0x5c, 0x64),
                    &rgba_from_hex(0xf6, 0x61, 0x51),
                    &rgba_from_hex(0x33, 0xd1, 0x7a),
                    &rgba_from_hex(0xe9, 0xad, 0x0c),
                    &rgba_from_hex(0x2a, 0x7b, 0xde),
                    &rgba_from_hex(0xc0, 0x61, 0xcb),
                    &rgba_from_hex(0x33, 0xc7, 0xde),
                    &rgba_from_hex(0xff, 0xff, 0xff),
                ],
            );

            obj.on_notify_dark();
            adw::StyleManager::default().connect_dark_notify(clone!(@weak obj => move |_| {
                glib::idle_add_local_once(clone!(@weak obj => move || {
                    obj.on_notify_dark();
                }));
            }));

            let status_expr = Self::Type::this_expression("container")
                .chain_property::<model::Container>("status");

            status_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| {
                        match status {
                            model::ContainerStatus::Running => "running",
                            _ => "not-running",
                        }
                    }
                ))
                .bind(&*self.stack, "visible-child-name", Some(obj));

            status_expr.watch(
                Some(obj),
                clone!(@weak obj => move || {
                    obj.action_set_enabled(
                        ACTION_START_OR_RESUME,
                        match obj
                            .container()
                            .filter(|c| c.status() == model::ContainerStatus::Running)
                        {
                            Some(container) => {
                                obj.setup_tty_connection(&container);
                                false
                            }
                            None => true,
                        },
                    );
                }),
            );
        }

        fn dispose(&self) {
            utils::ChildIter::from(self.obj().upcast_ref()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for Tty {
        fn grab_focus(&self) -> bool {
            self.terminal.grab_focus()
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.stack.allocate(width, height, baseline, None);
            if let Some(tx_tokio) = &*self.tx_tokio.borrow() {
                _ = tx_tokio.send(ExecInput::Resize {
                    columns: self.terminal.column_count() as usize,
                    rows: self.terminal.row_count() as usize,
                });
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct Tty(ObjectSubclass<imp::Tty>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Tty {
    fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    fn set_container(&self, value: Option<&model::Container>) {
        if self.container().as_ref() == value {
            return;
        }
        self.imp().container.set(value);
        self.notify("container");
    }

    pub(crate) fn font_scale(&self) -> f64 {
        self.imp().terminal.font_scale()
    }

    pub(crate) fn set_font_scale(&self, value: f64) {
        if self.font_scale() == value {
            return;
        }
        self.imp().terminal.set_font_scale(value);
    }

    fn setup_tty_connection(&self, container: &model::Container) {
        let imp = self.imp();

        let container = container.api().unwrap();

        let (tx_glib, rx_glib) = glib::MainContext::sync_channel::<Vec<u8>>(Default::default(), 5);

        rx_glib.attach(
            None,
            clone!(@weak self as obj => @default-return glib::Continue(false), move |buf| {
                obj.imp().terminal.feed(&buf);
                glib::Continue(true)
            }),
        );

        let (tx_tokio, mut rx_tokio) = tokio::sync::mpsc::unbounded_channel::<ExecInput>();
        imp.tx_tokio.replace(Some(tx_tokio.clone()));
        imp.terminal.connect_commit(move |_, data, _| {
            _ = tx_tokio.send(ExecInput::Data(data.as_bytes().to_vec()));
        });

        let width = imp.terminal.column_count();
        let height = imp.terminal.row_count();

        self.grab_focus();

        utils::do_async(
            async move {
                let opts = podman::opts::ExecCreateOpts::builder()
                    .attach_stderr(true)
                    .attach_stdout(true)
                    .attach_stdin(true)
                    .tty(true)
                    .command(["/bin/sh"])
                    .build();
                let exec = container.create_exec(&opts).await.unwrap();

                let opts = podman::opts::ExecStartOpts::builder().tty(true).build();
                let (mut reader, mut writer) = exec.start(&opts).await.unwrap().split();

                exec.resize(width as usize, height as usize).await?;

                loop {
                    match future::select(Box::pin(rx_tokio.recv()), reader.next()).await {
                        future::Either::Left((buf, _)) => match buf {
                            Some(buf) => match buf {
                                ExecInput::Data(buf) => {
                                    if let Err(e) = writer.write_all(&buf).await {
                                        log::error!("Error on writing to terminal: {e}");
                                        break;
                                    }
                                }
                                ExecInput::Resize { columns, rows } => {
                                    if let Err(e) = exec.resize(columns, rows).await {
                                        log::error!("Error on resizing terminal: {e}");
                                        break;
                                    }
                                }
                            },
                            None => break,
                        },
                        future::Either::Right((chunk, _)) => match chunk {
                            Some(chunk) => match chunk {
                                Ok(chunk) => {
                                    tx_glib.send(Vec::from(chunk)).unwrap();
                                }
                                Err(e) => {
                                    log::error!("Error on reading from terminal: {e}");
                                    break;
                                }
                            },
                            None => break,
                        },
                    }
                }

                // Close all processes.
                while writer.write_all(&[3]).await.is_ok() && writer.write_all(&[4]).await.is_ok() {
                }

                Ok(())
            },
            clone!(@weak self as obj => move |result: podman::Result<_>| {
                obj.emit_by_name::<()>("terminated", &[]);
                if result.is_err() {
                    utils::show_error_toast(
                        obj.upcast_ref(),
                        &gettext("Terminal error"),
                        &gettext("'/bin/sh' not found")
                    );
                }
            }),
        );
    }

    pub(crate) fn connect_terminated<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("terminated", true, move |values| {
            f(&values[0].get::<Self>().unwrap());

            None
        })
    }

    fn on_notify_dark(&self) {
        let style_context = self.style_context();

        let terminal = &*self.imp().terminal;
        terminal.set_color_background(&style_context.lookup_color("view_bg_color").unwrap());
        terminal.set_color_foreground(&style_context.lookup_color("view_fg_color").unwrap());
    }

    pub(crate) fn zoom_out(&self) {
        self.set_font_scale(self.font_scale() - 0.1);
    }

    pub(crate) fn zoom_in(&self) {
        self.set_font_scale(self.font_scale() + 0.1);
    }

    pub(crate) fn zoom_normal(&self) {
        self.set_font_scale(1.0);
    }

    fn copy(&self, format: vte4::Format) {
        let terminal = &*self.imp().terminal;
        if terminal.has_selection() {
            terminal.copy_clipboard_format(format);
        }
    }

    fn copy_plain(&self) {
        self.copy(vte4::Format::Text);
    }

    fn copy_html(&self) {
        self.copy(vte4::Format::Html);
    }

    fn paste(&self) {
        if let Some(display) = gdk::Display::default() {
            display.clipboard().read_text_async(
                gio::Cancellable::NONE,
                clone!(@weak self as obj => move |result| if let Some(text) = result
                    .ok()
                    .flatten()
                {
                    obj.imp().terminal.paste_text(text.as_str());
                }),
            );
        }
    }

    fn update_copy_actions(&self) {
        let has_selection = self.imp().terminal.has_selection();
        self.action_set_enabled(ACTION_COPY, has_selection);
        self.action_set_enabled(ACTION_COPY_HTML, has_selection);
    }
}

fn rgba_from_hex(r: i32, g: i32, b: i32) -> gdk::RGBA {
    gdk::RGBA::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 0.0)
}
