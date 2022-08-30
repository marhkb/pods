use std::collections::BTreeSet;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::PathBuf;

use futures::stream::BoxStream;
use futures::Future;
use futures::StreamExt;
use gettextrs::gettext;
use gtk::gio;
use gtk::glib;
use gtk::prelude::Cast;
use gtk::prelude::ListModelExt;
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

monad_boxed_type!(pub(crate) BoxedStringVec(Vec<String>) impls Debug, Default);
monad_boxed_type!(pub(crate) BoxedStringBTreeSet(BTreeSet<String>) impls Debug, Default);

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

pub(crate) fn root<W: glib::IsA<gtk::Widget>>(widget: &W) -> Window {
    widget.root().unwrap().downcast::<Window>().unwrap()
}

pub(crate) fn show_toast<W: glib::IsA<gtk::Widget>>(widget: &W, title: &str) {
    widget
        .root()
        .unwrap()
        .downcast::<Window>()
        .unwrap()
        .show_toast(
            &adw::Toast::builder()
                .title(title)
                .timeout(3)
                .priority(adw::ToastPriority::High)
                .build(),
        );
}

pub(crate) fn show_error_toast<W: glib::IsA<gtk::Widget>>(widget: &W, title: &str, msg: &str) {
    widget
        .root()
        .unwrap()
        .downcast::<Window>()
        .unwrap()
        .show_toast(
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
    run_stream_with_finish_handler(api_entity, stream_producer, glib_closure, |_| {
        glib::Continue(true)
    });
}

pub(crate) fn run_stream_with_finish_handler<A, P, I, F, X>(
    api_entity: A,
    stream_producer: P,
    glib_closure: F,
    finish_handler: X,
) where
    A: Send + 'static,
    for<'r> P: FnOnce(&'r A) -> BoxStream<'r, I> + Send + 'static,
    I: Send + 'static,
    F: FnMut(I) -> glib::Continue + 'static,
    X: FnMut(()) -> glib::Continue + 'static,
{
    let (tx_payload, rx_payload) = glib::MainContext::sync_channel::<I>(Default::default(), 5);
    let (tx_finish, rx_finish) = glib::MainContext::sync_channel::<()>(Default::default(), 1);

    rx_payload.attach(None, glib_closure);
    rx_finish.attach(None, finish_handler);

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

pub(crate) trait ToTypedListModel {
    fn to_typed_list_model<T>(self) -> TypedListModel<Self, T>
    where
        Self: Sized;
}

impl<M: glib::IsA<gio::ListModel>> ToTypedListModel for M {
    fn to_typed_list_model<T>(self) -> TypedListModel<Self, T>
    where
        Self: Sized,
    {
        TypedListModel::from(self)
    }
}

#[derive(Clone)]
pub(crate) struct TypedListModel<M, T> {
    model: M,
    _phantom: PhantomData<T>,
}

impl<M, T> From<M> for TypedListModel<M, T> {
    fn from(model: M) -> Self {
        Self {
            model,
            _phantom: PhantomData,
        }
    }
}

pub(crate) struct TypedListModelIter<M, T> {
    typed_list_store: TypedListModel<M, T>,
    index: u32,
}

impl<M, T> From<TypedListModel<M, T>> for TypedListModelIter<M, T> {
    fn from(typed_list_store: TypedListModel<M, T>) -> Self {
        TypedListModelIter {
            typed_list_store,
            index: 0,
        }
    }
}

impl<M: glib::IsA<gio::ListModel>, T: glib::IsA<glib::Object>> Iterator
    for TypedListModelIter<M, T>
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let t = self
            .typed_list_store
            .model
            .item(self.index)
            .and_then(|o| o.downcast::<T>().ok());
        self.index += 1;
        t
    }
}

impl<M, T> IntoIterator for TypedListModel<M, T>
where
    M: glib::IsA<gio::ListModel>,
    T: glib::IsA<glib::Object>,
{
    type Item = T;
    type IntoIter = TypedListModelIter<M, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.into()
    }
}
