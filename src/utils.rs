use std::cell::RefCell;
use std::fmt::Debug;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use ashpd::desktop::file_chooser::OpenFileRequest;
use ashpd::desktop::file_chooser::SaveFileRequest;
use ashpd::desktop::file_chooser::SelectedFiles;
use futures::Future;
use futures::StreamExt;
use futures::stream::BoxStream;
use gettextrs::gettext;
use gettextrs::ngettext;
use glib::clone;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone::Downgrade;

use crate::APPLICATION_OPTS;
use crate::config;

#[macro_export]
macro_rules! monad_boxed_type {
    ($vis:vis $boxed:ident($type:ty) $(impls $($trait:tt),+)? $(is $($prop:tt),+)?) => {
        paste::paste! {
            #[derive(Clone, glib::Boxed, $($($trait),+)?)]
            #[boxed_type(name = "" $boxed "", $($($prop),+)?)]
            $vis struct $boxed($type);
        }

        impl std::ops::Deref for $boxed {
            type Target = $type;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<$boxed> for $type {
            fn from(boxed: $boxed) -> Self {
                boxed.0
            }
        }

        impl From<$type> for $boxed {
            fn from(inner: $type) -> Self {
                Self(inner)
            }
        }
    };
}

pub(crate) fn config_dir() -> &'static PathBuf {
    &APPLICATION_OPTS.get().unwrap().config_dir
}

pub(crate) fn unix_socket_url() -> String {
    format!(
        "unix://{}",
        APPLICATION_OPTS
            .get()
            .unwrap()
            .unix_socket_path
            .to_str()
            .unwrap()
    )
}

#[derive(Debug)]
pub(crate) struct DesktopSettings(gio::Settings);

impl Default for DesktopSettings {
    fn default() -> Self {
        Self(gio::Settings::new("org.gnome.desktop.interface"))
    }
}

impl Deref for DesktopSettings {
    type Target = gio::Settings;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub(crate) struct PodsSettings(gio::Settings);

impl Default for PodsSettings {
    fn default() -> Self {
        Self(gio::Settings::new(config::APP_ID))
    }
}

impl Deref for PodsSettings {
    type Target = gio::Settings;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) fn human_friendly_duration(mut seconds: i64) -> String {
    let hours = seconds / (60 * 60);
    if hours > 0 {
        seconds %= 60 * 60;
        let minutes = seconds / 60;
        if minutes > 0 {
            seconds %= 60;
            gettext!("{} h {} min {} s", hours, minutes, seconds)
        } else {
            gettext!("{} h {} s", hours, seconds)
        }
    } else {
        let minutes = seconds / 60;
        if minutes > 0 {
            seconds %= 60;
            if seconds > 0 {
                gettext!("{} min {} s", minutes, seconds)
            } else {
                gettext!("{} min", minutes)
            }
        } else {
            gettext!("{} s", seconds)
        }
    }
}

pub(crate) fn timespan_now(timestamp: i64) -> glib::TimeSpan {
    glib::DateTime::now_utc()
        .unwrap()
        .difference(&glib::DateTime::from_unix_local(timestamp).unwrap())
}

pub(crate) fn human_friendly_timespan(timespan: glib::TimeSpan) -> String {
    let minutes = timespan.as_minutes();
    let hours = timespan.as_hours();

    if minutes < 1 {
        gettext("a few seconds")
    } else if minutes < 60 {
        ngettext!("{} minute", "{} minutes", minutes as u32, minutes)
    } else if hours < 24 {
        ngettext!("{} hour", "{} hours", hours as u32, hours)
    } else {
        let days = timespan.as_days();
        ngettext!("{} day", "{} days", days as u32, days)
    }
}

pub(crate) fn format_ago(timespan: glib::TimeSpan) -> String {
    // Translators: Example: {3 hours} ago, {a few seconds} ago
    gettext!("{} ago", human_friendly_timespan(timespan))
}

pub(crate) fn format_id(id: &str) -> String {
    id.chars().take(12).collect::<String>()
}

pub(crate) fn root<W: IsA<gtk::Widget>>(widget: &W) -> gtk::Window {
    widget.root().unwrap().downcast::<gtk::Window>().unwrap()
}

pub(crate) trait MaybeDefaultWidget {
    type Default: IsA<gtk::Widget>;

    fn default_widget(&self) -> Option<Self::Default> {
        None
    }
}

pub(crate) struct Dialog<'a, P, C> {
    parent: &'a P,
    content: &'a C,
    height: Option<i32>,
    width: Option<i32>,
    follows_content_size: Option<bool>,
}

impl<'a, C, P> Dialog<'a, C, P> {
    #[must_use]
    pub(crate) fn new(widget: &'a C, content: &'a P) -> Self {
        Self {
            parent: widget,
            content,
            height: None,
            width: None,
            follows_content_size: None,
        }
    }

    #[must_use]
    pub(crate) fn height(mut self, height: i32) -> Self {
        self.height = Some(height);
        self
    }

    #[must_use]
    pub(crate) fn follows_content_size(mut self, follows_content_size: bool) -> Self {
        self.follows_content_size = Some(follows_content_size);
        self
    }
}

impl<C, P> Dialog<'_, C, P>
where
    C: IsA<gtk::Widget>,
    P: IsA<gtk::Widget>,
{
    pub(crate) fn present(self) {
        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(self.content));

        let dialog = adw::Dialog::builder()
            .child(&toast_overlay)
            .width_request(360)
            .content_height(self.height.unwrap_or(-1))
            .content_width(self.width.unwrap_or(-1))
            .follows_content_size(self.follows_content_size.unwrap_or(false))
            .build();

        dialog.present(Some(self.parent));
    }
}

pub(crate) fn show_toast<W: IsA<gtk::Widget>>(widget: &W, title: impl Into<glib::GString>) {
    widget
        .ancestor(adw::ToastOverlay::static_type())
        .unwrap()
        .downcast::<adw::ToastOverlay>()
        .unwrap()
        .add_toast(
            adw::Toast::builder()
                .title(title)
                .timeout(3)
                .priority(adw::ToastPriority::High)
                .build(),
        );
}

pub(crate) fn show_error_toast<W: IsA<gtk::Widget>>(widget: &W, title: &str, msg: &str) {
    show_toast(widget, format!("{title}: {msg}"));
}

pub(crate) fn try_navigation_view<W: IsA<gtk::Widget>>(widget: &W) -> Option<adw::NavigationView> {
    widget
        .ancestor(adw::NavigationView::static_type())
        .and_downcast::<adw::NavigationView>()
}

pub(crate) fn navigation_view<W: IsA<gtk::Widget>>(widget: &W) -> adw::NavigationView {
    try_navigation_view(widget).unwrap()
}

pub(crate) fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
}

pub(crate) fn format_option<'a, T>(option: Option<T>) -> String
where
    T: AsRef<str> + 'a,
{
    option.map(|t| String::from(t.as_ref())).unwrap_or_else(||
            // Translators: This string will be shown when a property of an entity like an image is null.
            gettext("<none>"))
}

pub(crate) fn format_iter<'a, I, T>(iter: I, sep: &str) -> String
where
    I: IntoIterator<Item = &'a T>,
    T: AsRef<str> + ?Sized + 'a,
{
    format_option(format_iter_or_none(iter, sep))
}

pub(crate) fn format_iter_or_none<'a, I, T>(iter: I, sep: &str) -> Option<String>
where
    I: IntoIterator<Item = &'a T>,
    T: AsRef<str> + ?Sized + 'a,
{
    let mut iter = iter.into_iter();
    iter.next().map(|first| {
        Some(first)
            .into_iter()
            .chain(iter)
            .map(|some| some.as_ref())
            .collect::<Vec<_>>()
            .join(sep)
    })
}

// Function from https://gitlab.gnome.org/GNOME/fractal/-/blob/fractal-next/src/utils.rs
pub(crate) fn do_async<R, Fut, F>(tokio_fut: Fut, glib_closure: F)
where
    R: Send + 'static,
    Fut: Future<Output = R> + Send + 'static,
    F: FnOnce(R) + 'static,
{
    let handle = crate::runtime().spawn(tokio_fut);

    glib::MainContext::default().spawn_local_with_priority(Default::default(), async move {
        glib_closure(handle.await.unwrap());
    });
}

#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub(crate) struct AsyncObservers<R: Clone>(Rc<RefCell<Vec<Box<dyn Fn(R) + 'static>>>>);
impl<R: Clone> AsyncObservers<R> {
    fn new<F: Fn(R) + 'static>(glib_closure: F) -> Self {
        Self(Rc::new(RefCell::new(vec![Box::new(glib_closure)])))
    }

    pub(crate) fn add<F: Fn(R) + 'static>(&self, f: F) {
        self.0.borrow_mut().push(Box::new(f));
    }
}

pub(crate) fn do_async_with_observers<R, Fut, F>(
    tokio_fut: Fut,
    glib_closure: F,
) -> AsyncObservers<R>
where
    R: Clone + Send + Debug + 'static,
    Fut: Future<Output = R> + Send + 'static,
    F: Fn(R) + 'static,
{
    let observers = AsyncObservers::new(glib_closure);

    let handle = crate::runtime().spawn(tokio_fut);
    let (tx, rx) = tokio::sync::oneshot::channel::<R>();

    glib::spawn_future_local({
        let observers = observers.clone();
        async move {
            let r = rx.await.unwrap();
            observers
                .0
                .borrow_mut()
                .iter()
                .for_each(|fun| (*fun)(r.clone()));
        }
    });

    glib::MainContext::default().spawn_local_with_priority(Default::default(), async move {
        tx.send(handle.await.unwrap()).unwrap();
    });

    observers
}

pub(crate) fn run_stream<A, P, I, F>(api_entity: A, stream_producer: P, glib_closure: F)
where
    A: Send + 'static,
    for<'r> P: FnOnce(&'r A) -> BoxStream<'r, I> + Send + 'static,
    I: Send + 'static,
    F: FnMut(I) -> glib::ControlFlow + 'static,
{
    run_stream_with_finish_handler(api_entity, stream_producer, glib_closure, || {});
}

pub(crate) fn run_stream_with_finish_handler<A, P, I, F, X>(
    api_entity: A,
    stream_producer: P,
    mut glib_closure: F,
    mut finish_handler: X,
) where
    A: Send + 'static,
    for<'r> P: FnOnce(&'r A) -> BoxStream<'r, I> + Send + 'static,
    I: Send + 'static,
    F: FnMut(I) -> glib::ControlFlow + 'static,
    X: FnMut() + 'static,
{
    let (tx_payload, mut rx_payload) = tokio::sync::mpsc::channel(5);

    glib::spawn_future_local(async move {
        while let Some(i) = rx_payload.recv().await {
            if glib_closure(i) == glib::ControlFlow::Break {
                break;
            }
        }
        finish_handler();
    });

    crate::runtime().spawn(async move {
        let mut stream = stream_producer(&api_entity);
        while let Some(item) = stream.next().await {
            if tx_payload.send(item).await.is_err() {
                break;
            }
        }
    });
}

pub(crate) fn css_classes<W: IsA<gtk::Widget>>(widget: &W) -> Vec<String> {
    widget
        .css_classes()
        .iter()
        .map(glib::GString::to_string)
        .collect::<Vec<_>>()
}

pub(crate) struct ChildIter(Option<gtk::Widget>);
impl<W: IsA<gtk::Widget>> From<&W> for ChildIter {
    fn from(widget: &W) -> Self {
        Self(widget.first_child())
    }
}
impl Iterator for ChildIter {
    type Item = gtk::Widget;

    fn next(&mut self) -> Option<Self::Item> {
        let r = self.0.take();
        self.0 = r.as_ref().and_then(|widget| widget.next_sibling());
        r
    }
}

pub(crate) fn unparent_children<W: IsA<gtk::Widget>>(widget: &W) {
    ChildIter::from(widget).for_each(|child| child.unparent());
}

pub(crate) async fn show_open_file_dialog<W, F>(request: OpenFileRequest, widget: &W, op: F)
where
    W: IsA<gtk::Widget> + Downgrade<Weak = glib::WeakRef<W>>,
    F: Fn(SelectedFiles) + 'static,
{
    do_async(
        async move { request.send().await.and_then(|files| files.response()) },
        clone!(
            #[weak]
            widget,
            move |files| show_file_dialog(files, &widget, op)
        ),
    );
}

pub(crate) async fn show_save_file_dialog<W, F>(request: SaveFileRequest, widget: &W, op: F)
where
    W: IsA<gtk::Widget> + Downgrade<Weak = glib::WeakRef<W>>,
    F: Fn(SelectedFiles) + 'static,
{
    do_async(
        async move { request.send().await.and_then(|files| files.response()) },
        clone!(
            #[weak]
            widget,
            move |files| show_file_dialog(files, &widget, op)
        ),
    );
}

fn show_file_dialog<W, F>(files: Result<SelectedFiles, ashpd::Error>, widget: &W, op: F)
where
    W: IsA<gtk::Widget>,
    F: Fn(SelectedFiles),
{
    match files {
        Ok(files) => op(files),
        Err(e) => {
            if let ashpd::Error::Portal(ashpd::PortalError::Cancelled(_)) = e {
                show_error_toast(
                    widget,
                    &gettext("Error on open file dialog"),
                    &e.to_string(),
                );
            }
        }
    }
}

pub(crate) fn is_podman_id(name: &str) -> bool {
    name.len() == 64
        && name
            .chars()
            .all(|c| c.to_ascii_lowercase().is_ascii_hexdigit())
}

pub(crate) fn format_volume_name(name: &str) -> String {
    if is_podman_id(name) {
        format_id(name)
    } else {
        name.to_owned()
    }
}
