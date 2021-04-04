//! Alternative to [`futures_util::select_biased!`].

pub mod fused;
pub mod raw;

use std::future::Future;

use futures_util::{
	future::FusedFuture,
	stream::{FusedStream, Stream},
};
pub use macros::select;

use crate::PollOnce;

/// Workaround for specialization to enable passing [`Future`]s or [`Stream`]s
/// to be passed into [`select!`].
///
/// # Examples
/// ```
/// # futures_executor::block_on(async {
/// use std::task::Poll;
///
/// use allochronic_util::select::{PollFutures, PollStreams};
/// use futures_util::stream;
///
/// let mut future = async { 1 };
/// let mut stream = stream::iter(1..=3);
///
/// macro_rules! example {
/// 	($par:expr) => {
/// 		$par.__into_poll().await
/// 	};
/// }
///
/// assert_eq!(Poll::Ready(1), example!(future));
/// assert_eq!(Poll::Ready(Some(1)), example!(stream));
/// # });
/// ```
pub trait PollFutures: Sized {
	/// Shares method name with [`PollStreams::__into_poll`], returns
	/// [`PollOnce`] that can be polled exactly like
	/// [`PollStream`](raw::PollStream).
	fn __into_poll(self) -> PollOnce<Self>
	where
		Self: Future,
	{
		crate::poll(self)
	}

	/// Shares method name with [`PollStreams::__into_poll_fused`], returns
	/// [`PollFuture`](fused::PollFuture) that can be polled exactly like
	/// [`PollStream`](fused::PollStream).
	///
	/// # Examples
	/// ```
	/// # futures_executor::block_on(async {
	/// use std::task::Poll;
	///
	/// use allochronic_util::select::{PollFutures, PollStreams};
	/// use futures_util::{
	/// 	future::FutureExt,
	/// 	stream::{self, StreamExt},
	/// };
	///
	/// let mut future = async { 1 }.fuse();
	/// futures_util::pin_mut!(future);
	/// let mut stream = stream::iter(1..=3).fuse();
	///
	/// macro_rules! example {
	/// 	($par:expr) => {
	/// 		$par.__into_poll_fused().await
	/// 	};
	/// }
	///
	/// assert_eq!(Poll::Ready(Some(1)), example!(&mut future));
	/// assert_eq!(Poll::Ready(None), example!(future));
	///
	/// assert_eq!(Poll::Ready(Some(1)), example!(&mut stream));
	/// assert_eq!(Poll::Ready(Some(2)), example!(&mut stream));
	/// assert_eq!(Poll::Ready(Some(3)), example!(&mut stream));
	/// assert_eq!(Poll::Ready(None), example!(stream));
	/// # });
	/// ```
	fn __into_poll_fused(self) -> fused::PollFuture<Self>
	where
		Self: FusedFuture,
	{
		fused::PollFuture::new(self)
	}
}

impl<F: Future + Sized> PollFutures for F {}

/// Workaround for specialization to enable passing [`Future`]s or [`Stream`]s
/// to be passed into [`select!`].
///
/// # Examples
/// See [`PollFutures`].
pub trait PollStreams: Stream + Sized {
	/// Shares method name with [`PollFutures::__into_poll`], returns
	/// [`PollStream`](raw::PollStream) that can be polled exactly like
	/// [`PollOnce`].
	fn __into_poll(self) -> raw::PollStream<Self> { raw::PollStream::new(self) }

	/// Shares method name with [`PollFutures::__into_poll_fused`], returns
	/// [`PollStream`](fused::PollStream) that can be polled exactly like
	/// [`PollFuture`](fused::PollFuture).
	///
	/// # Examples
	/// See [`PollFutures::__into_poll_fused`].
	fn __into_poll_fused(self) -> fused::PollStream<Self>
	where
		Self: FusedStream,
	{
		fused::PollStream::new(self)
	}
}

impl<S: Stream + Sized> PollStreams for S {}
