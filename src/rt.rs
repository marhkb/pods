use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;

use futures::Stream;
use futures::StreamExt;
use futures::stream::BoxStream;
use gtk::glib;

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().unwrap())
}

pub(crate) struct Promise<Fut>(Fut);

impl<Fut> Promise<Fut> {
    pub(crate) fn new(future: Fut) -> Self {
        Self(future)
    }
}

impl<Fut> Promise<Fut>
where
    Fut: Future,
{
    pub(crate) fn block_on(self) -> Fut::Output {
        runtime().block_on(self.0)
    }
}

impl<Fut> Promise<Fut>
where
    Fut: Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    pub(crate) fn spawn(self) -> tokio::task::JoinHandle<Fut::Output> {
        runtime().spawn(self.0)
    }

    pub(crate) async fn exec(self) -> Fut::Output {
        self.spawn().await.expect("Failed to spawn future")
    }

    pub(crate) fn defer<F>(self, op: F)
    where
        F: FnOnce(Fut::Output) + 'static,
    {
        glib::spawn_future_local({
            let handle = self.spawn();
            async move {
                op(handle.await.unwrap());
            }
        });
    }
}

impl<Fut> Promise<Fut>
where
    Fut: Future + Send + 'static,
    Fut::Output: Clone + Send + 'static,
{
    pub(crate) fn defer_with_callbacks<F>(self, op: F) -> Callbacks<Fut::Output>
    where
        F: Fn(&Fut::Output) + 'static,
    {
        let callbacks = Callbacks::new(op);

        glib::spawn_future_local({
            let handle = self.spawn();
            let callbacks = callbacks.clone();
            async move {
                let output = handle.await.unwrap();
                callbacks
                    .0
                    .borrow_mut()
                    .iter()
                    .for_each(|fun| (*fun)(&output));
            }
        });

        callbacks
    }
}

#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub(crate) struct Callbacks<T: Clone>(Rc<RefCell<Vec<Box<dyn Fn(&T) + 'static>>>>);
impl<T: Clone> Callbacks<T> {
    pub(crate) fn new<F: Fn(&T) + 'static>(op: F) -> Self {
        Self(Rc::new(RefCell::new(vec![Box::new(op)])))
    }

    pub(crate) fn add<F: Fn(&T) + 'static>(&self, op: F) {
        self.0.borrow_mut().push(Box::new(op));
    }
}

pub(crate) struct Pipe<A, P> {
    api: A,
    producer: P,
}

impl<A, P, I> Pipe<A, P>
where
    A: Send + 'static,
    for<'r> P: FnOnce(&'r A) -> BoxStream<'r, I> + Send + 'static,
    I: Send + 'static,
{
    pub(crate) fn new(api: A, producer: P) -> Self {
        Pipe { api, producer }
    }

    pub(crate) fn stream(self) -> impl Stream<Item = I> {
        let (tx, rx) = tokio::sync::mpsc::channel(10);

        Promise::new(async move {
            let mut stream = (self.producer)(&self.api);

            while let Some(item) = stream.next().await {
                if tx.send(item).await.is_err() {
                    break;
                }
            }
        })
        .spawn();

        tokio_stream::wrappers::ReceiverStream::new(rx)
    }

    pub(crate) fn on_next<F>(self, mut op: F) -> PipeFinish
    where
        F: FnMut(I) -> glib::ControlFlow + 'static,
    {
        glib::spawn_future_local({
            let mut stream = self.stream();
            async move {
                while let Some(item) = stream.next().await {
                    if op(item) == glib::ControlFlow::Break {
                        break;
                    }
                }
            }
        })
        .into()
    }
}

pub(crate) struct PipeFinish(glib::JoinHandle<()>);
impl From<glib::JoinHandle<()>> for PipeFinish {
    fn from(value: glib::JoinHandle<()>) -> Self {
        Self(value)
    }
}
impl PipeFinish {
    pub(crate) fn on_finish<F>(self, mut op: F)
    where
        F: FnMut() + 'static,
    {
        glib::spawn_future_local(async move {
            self.0.await.unwrap();
            op();
        });
    }
}
