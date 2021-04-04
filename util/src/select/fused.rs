//! See [`PollFuture`] and [`PollStream`].

use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

use futures_util::{future::FusedFuture, stream::FusedStream};
use pin_project::pin_project;

/// [`Poll`](std::future::Future::poll)s a [`FusedFuture`] a single time,
/// returning [`Poll<Option<T>>`]. Used in combination with
/// [`PollFutures`](super::PollFutures::__into_poll_fused).
///
/// Will always return [`None`] if [`is_terminated`](FusedFuture::is_terminated)
/// returns `true`.
///
/// # Examples
/// ```
/// # futures_executor::block_on(async {
/// use std::task::Poll;
///
/// use allochronic_util::select::fused;
/// use futures_util::future::{self, FutureExt};
///
/// // `ready` already produces a fused `Future`, `fuse()` here is just to showcase
/// let mut future = future::ready(1).fuse();
/// assert_eq!(
/// 	Poll::Ready(Some(1)),
/// 	fused::PollFuture::new(&mut future).await
/// );
/// assert_eq!(Poll::Ready(None), fused::PollFuture::new(&mut future).await);
/// // doesn't panic even if you keep pulling
/// assert_eq!(Poll::Ready(None), fused::PollFuture::new(future).await);
/// # });
/// ```
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
#[derive(Debug)]
pub struct PollFuture<F: FusedFuture>(#[pin] Option<F>);

impl<F: FusedFuture> PollFuture<F> {
	/// Builds a new [`PollFuture`].
	pub fn new(future: F) -> Self {
		Self(Some(future))
	}
}

impl<F: FusedFuture> Future for PollFuture<F> {
	type Output = Poll<Option<F::Output>>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Poll::Ready(match self.as_mut().project().0.as_pin_mut() {
			Some(mut future) => {
				// according to the documentation, the status of `is_terminated` can change
				// anytime, we shouldn't pull if it is terminated
				if future.is_terminated() {
					self.as_mut().project().0.set(None);
					Poll::Ready(None)
				} else {
					future.as_mut().poll(cx).map(Some)
				}
			}
			None => Poll::Ready(None),
		})
	}
}

/// [`Poll`](std::stream::Stream::poll_next)s a [`FusedStream`] a single time,
/// returning [`Poll<Option<T>>`]. Used in combination with
/// [`PollStreams`](super::PollStreams::__into_poll_fused).
///
/// Will always return [`None`] if [`is_terminated`](FusedStream::is_terminated)
/// returns `true`.
///
/// # Examples
/// ```
/// # futures_executor::block_on(async {
/// use std::task::Poll;
///
/// use allochronic_util::select::fused;
/// use futures_util::stream::{self, StreamExt};
///
/// let mut stream = stream::iter(1..=3).fuse();
/// assert_eq!(
/// 	Poll::Ready(Some(1)),
/// 	fused::PollStream::new(&mut stream).await
/// );
/// assert_eq!(
/// 	Poll::Ready(Some(2)),
/// 	fused::PollStream::new(&mut stream).await
/// );
/// assert_eq!(
/// 	Poll::Ready(Some(3)),
/// 	fused::PollStream::new(&mut stream).await
/// );
/// assert_eq!(Poll::Ready(None), fused::PollStream::new(&mut stream).await);
/// // doesn't panic even if you keep pulling
/// assert_eq!(Poll::Ready(None), fused::PollStream::new(stream).await);
/// # });
/// ```
#[must_use = "streams do nothing unless polled"]
#[pin_project]
#[derive(Debug)]
pub struct PollStream<S: FusedStream>(#[pin] Option<S>);

impl<S: FusedStream> PollStream<S> {
	/// Builds a new [`PollStream`].
	pub fn new(stream: S) -> Self {
		Self(Some(stream))
	}
}

impl<S: FusedStream> Future for PollStream<S> {
	type Output = Poll<Option<S::Item>>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Poll::Ready(match self.as_mut().project().0.as_pin_mut() {
			Some(mut stream) => {
				// according to the documentation, the status of `is_terminated` can change
				// anytime, we shouldn't pull if it is terminated
				if stream.is_terminated() {
					self.as_mut().project().0.set(None);
					Poll::Ready(None)
				} else {
					stream.as_mut().poll_next(cx)
				}
			}
			None => Poll::Ready(None),
		})
	}
}
