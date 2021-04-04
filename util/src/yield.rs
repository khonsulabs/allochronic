//! See [`yield`] and [`Yield`].

use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

/// Yields to the executor once.
///
/// # Examples
/// ```
/// # futures_executor::block_on(async {
/// use std::task::Poll;
///
/// use allochronic_util::{poll, r#yield};
///
/// let mut future = async {
/// 	r#yield().await;
/// 	1
/// };
/// futures_util::pin_mut!(future);
///
/// assert_eq!(Poll::Pending, poll(&mut future).await);
/// assert_eq!(Poll::Ready(1), poll(future).await);
/// # });
/// ```
pub const fn r#yield() -> Yield { Yield::new() }

/// Yields to the executor once. See [`yield`] for easier usage.
///
/// # Examples
/// ```
/// # futures_executor::block_on(async {
/// use std::task::Poll;
///
/// use allochronic_util::{poll, Yield};
///
/// let mut future = async {
/// 	Yield::new().await;
/// 	1
/// };
/// futures_util::pin_mut!(future);
///
/// assert_eq!(Poll::Pending, poll(&mut future).await);
/// assert_eq!(Poll::Ready(1), poll(future).await);
/// # });
/// ```
#[allow(missing_copy_implementations)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Debug)]
pub struct Yield(bool);

impl Yield {
	/// Builds a new [`Yield`].
	pub const fn new() -> Self { Self(false) }
}

impl Future for Yield {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		if self.0 {
			Poll::Ready(())
		} else {
			self.0 = true;
			// the task is ready immediately
			cx.waker().wake_by_ref();
			Poll::Pending
		}
	}
}
