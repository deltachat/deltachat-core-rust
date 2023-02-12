//! Futures extensions to track current context ID.

use pin_project_lite::pin_project;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread_local;

thread_local! {
    static THREAD_CONTEXT_ID: RefCell<u32> = RefCell::new(0);
}

pub(crate) struct ContextIdGuard {
    previous: u32,
}

pub(crate) fn current_context_id() -> u32 {
    THREAD_CONTEXT_ID.with(|context_id| *context_id.borrow())
}

impl ContextIdGuard {
    fn new(context_id: u32) -> Self {
        let previous = THREAD_CONTEXT_ID.with(|prev_context_id| {
            let ret = *prev_context_id.borrow();
            *prev_context_id.borrow_mut() = context_id;
            ret
        });
        Self { previous }
    }
}

impl Drop for ContextIdGuard {
    fn drop(&mut self) {
        THREAD_CONTEXT_ID.with(|context_id| {
            *context_id.borrow_mut() = self.previous;
        })
    }
}

pin_project! {
    /// A future with attached context ID.
    #[derive(Debug, Clone)]
    pub struct ContextIdFuture<F> {
        context_id: u32,
        #[pin]
        future: F,
    }
}

impl<F> ContextIdFuture<F> {
    /// Wraps a future.
    pub fn new(context_id: u32, future: F) -> Self {
        Self { context_id, future }
    }
}

impl<F: Future> Future for ContextIdFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let context_id = self.context_id;

        let this = self.project();
        let _guard = ContextIdGuard::new(context_id);
        this.future.poll(cx)
    }
}

/// Future extension to bind context ID.
pub trait ContextIdFutureExt: Sized {
    /// Binds context ID to the future.
    fn bind_context_id(self, context_id: u32) -> ContextIdFuture<Self> {
        ContextIdFuture::new(context_id, self)
    }

    /// Binds current context ID to the future.
    fn bind_current_context_id(self) -> ContextIdFuture<Self> {
        self.bind_context_id(current_context_id())
    }
}

impl<F> ContextIdFutureExt for F where F: Future {}
