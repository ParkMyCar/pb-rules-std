use futures::{Stream, StreamExt};
use std::cell::RefCell;
use std::future::Future;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable, Waker};

use crate::StdRules;
use crate::pb::rules::types::Provider;

pub mod http;
pub mod types;

impl super::exports::pb::rules::rules::Guest for StdRules {
    type Rule = StdRule;
    type RuleFuture = FutureAdapter<Vec<crate::pb::rules::types::Provider>>;

    fn rule_set() -> super::_rt::Vec<(super::_rt::String, super::exports::pb::rules::rules::Rule)> {
        let location = super::pb::rules::logging::Location {
            file_path: None,
            line: None,
        };
        super::pb::rules::logging::event(
            super::pb::rules::logging::Level::Info,
            "rule set",
            &location,
            &[],
        );

        vec![(
            "http".into(),
            super::exports::pb::rules::rules::Rule::new(StdRule::default()),
        )]
    }
}

#[derive(Default)]
pub struct StdRule;

impl super::exports::pb::rules::rules::GuestRule for StdRule {
    fn name() -> super::_rt::String {
        "TODO".into()
    }

    fn run(
        &self,
        attrs: super::_rt::Vec<(super::_rt::String, crate::pb::rules::types::Attribute)>,
        context: crate::pb::rules::context::Ctx,
    ) -> super::exports::pb::rules::rules::RuleFuture {
        let location = super::pb::rules::logging::Location {
            file_path: None,
            line: None,
        };
        super::pb::rules::logging::event(
            super::pb::rules::logging::Level::Info,
            "called std rule!",
            &location,
            &[],
        );

        let future = Box::pin(async move {
            let request = crate::pb::rules::http::Request {
                url: "https://jsonplaceholder.typicode.com/comments".into(),
                headers: vec![],
            };

            let mut body = Vec::new();
            let response = context.actions().http().get(&request);
            let mut body_stream = StreamWrapper::new(response.body());
            while let Some(val) = body_stream.next().await {
                let location = super::pb::rules::logging::Location {
                    file_path: None,
                    line: None,
                };
                super::pb::rules::logging::event(
                    super::pb::rules::logging::Level::Info,
                    &format!("chunk {}", val.len()),
                    &location,
                    &[],
                );
                body.extend_from_slice(&val[..]);
            }
            let msg = String::from_utf8_lossy(&body[..]);

            let location = super::pb::rules::logging::Location {
                file_path: None,
                line: None,
            };
            super::pb::rules::logging::event(
                super::pb::rules::logging::Level::Info,
                &format!("here here here {}", msg),
                &location,
                &[],
            );

            vec![]
        });

        let adapter = FutureAdapter {
            inner: RefCell::new(future),
        };
        super::exports::pb::rules::rules::RuleFuture::new(adapter)
    }
}

pub struct FutureAdapter<T> {
    inner: RefCell<Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>>>,
}

impl<T: 'static> FutureAdapter<T> {
    fn poll(&self, waker: crate::exports::pb::rules::rules::Waker) -> std::task::Poll<T> {
        let waker = WakerAdapter2::new(waker).waker();
        let mut context = std::task::Context::from_waker(&waker);
        let mut inner = self.inner.borrow_mut();
        inner.as_mut().poll(&mut context)
    }
}

impl super::exports::pb::rules::rules::GuestRuleFuture for FutureAdapter<Vec<Provider>> {
    fn poll(
        &self,
        waker: crate::exports::pb::rules::rules::Waker,
    ) -> crate::exports::pb::rules::rules::RulePoll {
        match self.poll(waker) {
            std::task::Poll::Ready(result) => {
                crate::exports::pb::rules::rules::RulePoll::Ready(result)
            }
            std::task::Poll::Pending => crate::exports::pb::rules::rules::RulePoll::Pending,
        }
    }
}

pub struct StreamWrapper {
    inner: crate::pb::rules::http::BodyStream,
}

impl StreamWrapper {
    pub fn new(inner: crate::pb::rules::http::BodyStream) -> Self {
        StreamWrapper { inner }
    }
}

impl Stream for StreamWrapper {
    type Item = Vec<u8>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let waker = cx.waker().data() as *const ();

        let location = super::pb::rules::logging::Location {
            file_path: None,
            line: None,
        };
        super::pb::rules::logging::event(
            super::pb::rules::logging::Level::Info,
            &format!("polling stream {:?}", waker),
            &location,
            &[],
        );

        let waker = waker as *const crate::exports::pb::rules::rules::Waker;
        let waker = unsafe { &*waker };
        let waker = waker.clone();

        match self.as_ref().inner.poll_next(waker) {
            crate::pb::rules::http::BodyPoll::Pending => std::task::Poll::Pending,
            crate::pb::rules::http::BodyPoll::Ready(val) => std::task::Poll::Ready(val),
        }
    }
}

pub struct WakerAdapter {
    inner: crate::exports::pb::rules::rules::Waker,
}

impl WakerAdapter {
    fn new(inner: crate::exports::pb::rules::rules::Waker) -> Self {
        WakerAdapter { inner }
    }
}

impl std::task::Wake for WakerAdapter {
    fn wake(self: std::sync::Arc<Self>) {
        self.inner.wake();
    }
}

static ADAPTER_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    raw_waker_adapter_clone,
    raw_waker_adapter_wake,
    raw_waker_adapter_wake_by_ref,
    raw_waker_adapter_drop,
);

pub struct WakerAdapter2 {
    inner: Arc<crate::exports::pb::rules::rules::Waker>,
}

impl WakerAdapter2 {
    pub fn new(waker: crate::exports::pb::rules::rules::Waker) -> Self {
        WakerAdapter2 {
            inner: Arc::new(waker),
        }
    }

    pub fn waker(self) -> std::task::Waker {
        let waker = Arc::into_raw(self.inner) as *const ();

        let location = super::pb::rules::logging::Location {
            file_path: None,
            line: None,
        };
        super::pb::rules::logging::event(
            super::pb::rules::logging::Level::Info,
            &format!("{:?}", waker),
            &location,
            &[],
        );

        unsafe { std::task::Waker::new(waker, &ADAPTER_WAKER_VTABLE) }
    }
}

unsafe fn raw_waker_adapter_clone(waker: *const ()) -> RawWaker {
    unsafe {
        Arc::increment_strong_count(waker as *const crate::exports::pb::rules::rules::Waker);
    }
    RawWaker::new(waker as *const (), &ADAPTER_WAKER_VTABLE)
}

unsafe fn raw_waker_adapter_wake(waker: *const ()) {
    let waker = unsafe { Arc::from_raw(waker as *const crate::exports::pb::rules::rules::Waker) };
    waker.wake();
}

unsafe fn raw_waker_adapter_wake_by_ref(waker: *const ()) {
    let waker = unsafe {
        ManuallyDrop::new(Arc::from_raw(
            waker as *const crate::exports::pb::rules::rules::Waker,
        ))
    };
    waker.wake();
}

unsafe fn raw_waker_adapter_drop(waker: *const ()) {
    unsafe {
        Arc::decrement_strong_count(waker as *const crate::exports::pb::rules::rules::Waker);
    }
}
