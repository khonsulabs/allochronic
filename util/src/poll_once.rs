//! See [`poll`] and [`PollOnce`].

use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

use pin_project::pin_project;

/// [`Poll`](std::future::Future::poll)s a [`Future`] a single time, returning
/// [`Poll`].
///
/// # Examples
/// ```
/// # futures_executor::block_on(async {
/// use std::task::Poll;
///
/// use allochronic_util::poll;
///
/// let mut future = async { 1 };
/// assert_eq!(Poll::Ready(1), poll(future).await);
/// # });
/// ```
pub fn poll<F: Future>(future: F) -> PollOnce<F> { PollOnce::new(future) }

/// [`Poll`](std::future::Future::poll)s a [`Future`] a single time, returning
/// [`Poll`]. See [`poll`] for easier usage.
///
/// # Examples
/// ```
/// # futures_executor::block_on(async {
/// use std::task::Poll;
///
/// use allochronic_util::PollOnce;
///
/// let mut future = async { 1 };
/// assert_eq!(Poll::Ready(1), PollOnce::new(future).await);
/// # });
/// ```
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
#[derive(Debug)]
pub struct PollOnce<F: Future>(#[pin] F);

impl<F: Future> PollOnce<F> {
	/// Builds a new [`PollOnce`].
	pub fn new(future: F) -> Self { Self(future) }
}

impl<F: Future> Future for PollOnce<F> {
	type Output = Poll<F::Output>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Poll::Ready(self.project().0.poll(cx))
	}
}
