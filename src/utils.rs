use std::collections::BTreeSet;
use std::marker::PhantomData;
use std::ops::Deref;

use futures::Future;
use futures::Stream;
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
use crate::RUNTIME;

#[macro_export]
macro_rules! monad_boxed_type {
    ($vis:vis $boxed:ident($type:ty) $(impls $($trait:tt),+)? $(is $($prop:tt),+)?) => {
        paste::paste! {
            #[derive(Clone, PartialEq, glib::Boxed, $($($trait),+)?)]
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
                .title(&format!("{title}:{msg}"))
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

pub(crate) fn run_stream<S, I, F>(mut stream: S, glib_closure: F)
where
    S: Stream<Item = I> + Send + Unpin + 'static,
    I: Send + 'static,
    F: FnMut(I) -> glib::Continue + 'static,
{
    let (sender, receiver) = glib::MainContext::sync_channel::<I>(Default::default(), 5);

    receiver.attach(None, glib_closure);

    RUNTIME.spawn(async move {
        while let Some(item) = stream.next().await {
            if sender.send(item).is_err() {
                break;
            }
        }
    });
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
