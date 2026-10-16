//! A tiny single-threaded executor for the testkit's own tests.

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

/// Drives a future that never waits on anything external to completion.
pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
    }
}
