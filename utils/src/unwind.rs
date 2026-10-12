//! Turning a panicking future into a value the caller can answer.
//!
//! A handler that panics would otherwise unwind through the connection task and take
//! the connection with it: no response, and every request pipelined behind it lost
//! too. Catching the panic turns that into an ordinary `Result` the dispatcher can
//! answer `500` to, on a connection that is still synchronised — a panic is a fault
//! in the handler, not in the framing, so the stream is still exactly where the
//! request left it.

use std::{
    any::Any,
    future::Future,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    task::{Context, Poll},
};

/// What a caught panic carries: the argument the handler panicked with.
///
/// `panic!("no such ticket")` leaves a `&'static str` in here and
/// `panic!("{id} is gone")` a `String`, so a caller that wants the message
/// downcasts to both. Most callers want neither — the default panic hook has already
/// printed the message and the location by the time this is returned.
pub type PanicPayload = Box<dyn Any + Send>;

/// A future that reports a panic instead of propagating it.
///
/// [`catch_unwind`] guards a closure, and a handler is a future: the panic happens on
/// whichever poll runs the offending line, long after any closure wrapping the *call*
/// has returned. So what is guarded here is each individual poll, which is also how
/// Tokio catches panics inside spawned tasks.
///
/// Spawning is in fact the other way to get this, through `JoinError::is_panic`, and
/// it is not usable here: `tokio::spawn` needs a `'static` future, while a handler
/// borrows the request buffer for as long as it runs. Making it `'static` would mean
/// copying every request to serve the exceptional case.
///
/// # `Unpin`
///
/// The bound is what keeps this free of `unsafe`. A self-referential future would
/// have to be projected through a `Pin`, which cannot be done safely by hand; an
/// [`Unpin`] one can simply be re-pinned on each poll. Every handler reaches this as
/// `Pin<Box<dyn Future>>` — the shape the route table stores — which is [`Unpin`]
/// whatever it wraps, so the bound costs callers nothing.
pub struct CatchUnwind<F> {
    inner: F,
}

impl<F> CatchUnwind<F> {
    /// Wraps `future` so that a panic from it is returned rather than propagated.
    pub fn new(future: F) -> Self {
        CatchUnwind { inner: future }
    }
}

impl<F: Future + Unpin> Future for CatchUnwind<F> {
    type Output = Result<F::Output, PanicPayload>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = &mut self.get_mut().inner;

        match catch_unwind(AssertUnwindSafe(|| Pin::new(inner).poll(context))) {
            Ok(poll) => poll.map(Ok),
            Err(payload) => Poll::Ready(Err(payload)),
        }
    }
}

/// Runs `future`, returning `Err` if it panics.
///
/// The spelling most callers want: [`CatchUnwind::new`] with the turn of phrase at
/// the call site rather than the type.
///
/// # Examples
///
/// ```
/// use utils::unwind::catch;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let answered = catch(Box::pin(async { "ok" })).await;
/// assert_eq!(answered.unwrap(), "ok");
///
/// // The panic is caught rather than unwinding into this function. It is still
/// // printed first: the default hook runs before anything unwinds.
/// let panicked = catch(Box::pin(async { panic!("no such ticket") })).await;
/// assert!(panicked.is_err());
/// # }
/// ```
pub async fn catch<F: Future + Unpin>(future: F) -> Result<F::Output, PanicPayload> {
    CatchUnwind::new(future).await
}

#[cfg(test)]
#[path = "unwind_tests.rs"]
mod tests;
