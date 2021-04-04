//! See [`PollStream`].

use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

use futures_util::stream::Stream;
use pin_project::pin_project;

/// [`Poll`](std::stream::Stream::poll_next)s a [`Stream`] a single time,
/// returning [`Poll`]. Used in combination with
/// [`PollStreams`](crate::select::PollStreams::__into_poll).
///
/// # Examples
/// ```
/// # futures_executor::block_on(async {
/// use std::task::Poll;
///
/// use allochronic_util::select::raw;
/// use futures_util::stream;
///
/// let mut stream = stream::iter(1..=3);
/// assert_eq!(
/// 	Poll::Ready(Some(1)),
/// 	raw::PollStream::new(&mut stream).await
/// );
/// assert_eq!(
/// 	Poll::Ready(Some(2)),
/// 	raw::PollStream::new(&mut stream).await
/// );
/// assert_eq!(
/// 	Poll::Ready(Some(3)),
/// 	raw::PollStream::new(&mut stream).await
/// );
/// assert_eq!(Poll::Ready(None), raw::PollStream::new(stream).await);
/// # });
/// ```
#[must_use = "streams do nothing unless polled"]
#[pin_project]
#[derive(Debug)]
pub struct PollStream<S: Stream>(#[pin] S);

impl<S: Stream> PollStream<S> {
	/// Builds a new [`PollStream`].
	pub fn new(stream: S) -> Self { Self(stream) }
}

impl<S: Stream> Future for PollStream<S> {
	type Output = Poll<Option<S::Item>>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Poll::Ready(self.project().0.poll_next(cx))
	}
}
