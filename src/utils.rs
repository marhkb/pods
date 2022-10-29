use std::ops::Deref;
use std::path::PathBuf;

use futures::stream::BoxStream;
use futures::Future;
use futures::StreamExt;
use gettextrs::gettext;
use gettextrs::ngettext;
use gtk::gio;
use gtk::glib;
use gtk::prelude::Cast;
use gtk::prelude::StaticType;
use gtk::traits::WidgetExt;

use crate::config;
use crate::view;
use crate::window::Window;
use crate::APPLICATION_OPTS;
use crate::RUNTIME;

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

pub(crate) fn root<W: glib::IsA<gtk::Widget>>(widget: &W) -> Window {
    widget.root().unwrap().downcast::<Window>().unwrap()
}

pub(crate) fn show_toast<W: glib::IsA<gtk::Widget>>(widget: &W, title: &str) {
    root(widget).show_toast(
        &adw::Toast::builder()
            .title(title)
            .timeout(3)
            .priority(adw::ToastPriority::High)
            .build(),
    );
}

pub(crate) fn show_error_toast<W: glib::IsA<gtk::Widget>>(widget: &W, title: &str, msg: &str) {
    root(widget).show_toast(
        &adw::Toast::builder()
            .title(&format!("{title}: {msg}"))
            .timeout(3)
            .priority(adw::ToastPriority::High)
            .build(),
    );
}

pub(crate) fn find_leaflet_overlay<W: glib::IsA<gtk::Widget>>(widget: &W) -> view::LeafletOverlay {
    leaflet_overlay(
        &widget
            .ancestor(adw::Leaflet::static_type())
            .unwrap()
            .downcast::<adw::Leaflet>()
            .unwrap(),
    )
}

pub(crate) fn leaflet_overlay(leaflet: &adw::Leaflet) -> view::LeafletOverlay {
    leaflet
        .child_by_name("overlay")
        .unwrap()
        .downcast::<view::LeafletOverlay>()
        .unwrap()
}

pub(crate) fn find_parent_leaflet_overlay<W: glib::IsA<gtk::Widget>>(
    widget: &W,
) -> view::LeafletOverlay {
    widget
        .ancestor(view::LeafletOverlay::static_type())
        .unwrap()
        .downcast::<view::LeafletOverlay>()
        .unwrap()
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

pub(crate) fn format_iter<'a, I, T: ?Sized>(iter: I, sep: &str) -> String
where
    I: IntoIterator<Item = &'a T>,
    T: AsRef<str> + 'a,
{
    format_option(format_iter_or_none(iter, sep))
}

pub(crate) fn format_iter_or_none<'a, I, T: ?Sized + 'a>(iter: I, sep: &str) -> Option<String>
where
    I: IntoIterator<Item = &'a T>,
    T: AsRef<str> + 'a,
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
    let handle = RUNTIME.spawn(tokio_fut);

    glib::MainContext::default().spawn_local_with_priority(Default::default(), async move {
        glib_closure(handle.await.unwrap());
    });
}

pub(crate) fn run_stream<A, P, I, F>(api_entity: A, stream_producer: P, glib_closure: F)
where
    A: Send + 'static,
    for<'r> P: FnOnce(&'r A) -> BoxStream<'r, I> + Send + 'static,
    I: Send + 'static,
    F: FnMut(I) -> glib::Continue + 'static,
{
    run_stream_with_finish_handler(api_entity, stream_producer, glib_closure, || {});
}

pub(crate) fn run_stream_with_finish_handler<A, P, I, F, X>(
    api_entity: A,
    stream_producer: P,
    glib_closure: F,
    mut finish_handler: X,
) where
    A: Send + 'static,
    for<'r> P: FnOnce(&'r A) -> BoxStream<'r, I> + Send + 'static,
    I: Send + 'static,
    F: FnMut(I) -> glib::Continue + 'static,
    X: FnMut() + 'static,
{
    let (tx_payload, rx_payload) = glib::MainContext::sync_channel::<I>(Default::default(), 5);
    let (tx_finish, rx_finish) = glib::MainContext::sync_channel::<()>(Default::default(), 1);

    rx_payload.attach(None, glib_closure);
    rx_finish.attach(None, move |_| {
        finish_handler();
        glib::Continue(false)
    });

    RUNTIME.spawn(async move {
        let mut stream = stream_producer(&api_entity);
        while let Some(item) = stream.next().await {
            if tx_payload.send(item).is_err() {
                break;
            }
        }
        tx_finish.send(()).unwrap();
    });
}

pub(crate) struct ChildIter(Option<gtk::Widget>);
impl<W: glib::IsA<gtk::Widget>> From<&W> for ChildIter {
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
