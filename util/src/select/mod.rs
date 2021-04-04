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
	fn __into_poll(self) -> raw::PollStream<Self> {
		raw::PollStream::new(self)
	}

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

#[cfg(test)]
mod test {
	use std::task::Poll;

	use futures_executor::block_on;
	use futures_util::{
		pin_mut,
		stream::{self, poll_fn},
		FutureExt, StreamExt,
	};
	use stream::iter;

	use crate::select;

	#[test]
	#[should_panic = "`async fn` resumed after completion"]
	fn select_future() {
		block_on(async {
			let future = async { 1_usize };
			pin_mut!(future);

			while select![
				result: &mut future => { assert_eq!(1, result); true },
				r#yield => unreachable!(),
			] {}
		});
	}

	#[test]
	#[should_panic = "`async fn` resumed after completion"]
	fn select_futures() {
		block_on(async {
			let future1 = async { 1_usize };
			let future2 = async { 2_usize };

			pin_mut!(future1);
			pin_mut!(future2);

			#[allow(unreachable_code)]
			while select![
				result: &mut future1 => { assert_eq!(1, result); true },
				_: &mut future2 => unreachable!(),
				r#yield => unreachable!(),
			] {}
		});
	}

	#[test]
	fn select_stream() {
		block_on(async {
			let mut counter = 0;

			let mut stream = iter(1_usize..=10);

			while let Some(result) = select![
				result: &mut stream => result,
				r#yield => unreachable!(),
			] {
				counter += 1;
				assert_eq!(counter, result);
			}

			assert_eq!(counter, 10);
		});
	}

	#[test]
	fn select_streams() {
		block_on(async {
			let mut counter = 0;

			let mut stream1 = iter(1_usize..=10);
			let mut stream2 = iter(11..=20);

			while let Some(result) = select![
				result: &mut stream1 => result,
				result: &mut stream2 => result,
				r#yield => unreachable!(),
			] {
				counter += 1;
				assert_eq!(counter, result);
			}

			assert_eq!(counter, 10);
		});
	}

	#[test]
	fn select_future_fused() {
		block_on(async {
			let mut counter = 0;

			let future = async { 1_usize }.fuse();
			pin_mut!(future);

			while let Some(result) = select![
				result: &mut future => Some(result),
				r#yield => unreachable!(),
				complete => None,
			] {
				counter += 1;
				assert_eq!(counter, result);
			}

			assert_eq!(counter, 1);
		});
	}

	#[test]
	fn select_futures_fused() {
		block_on(async {
			let mut counter = 0;

			let future1 = async { 1_usize }.fuse();
			let future2 = async { 2 }.fuse();

			pin_mut!(future1);
			pin_mut!(future2);

			while let Some(result) = select![
				result: &mut future1 => Some(result),
				result: &mut future2 => Some(result),
				r#yield => unreachable!(),
				complete => None,
			] {
				counter += 1;
				assert_eq!(counter, result);
			}

			assert_eq!(counter, 2);
		});
	}

	#[test]
	fn select_stream_fused() {
		block_on(async {
			let mut counter = 0;

			let mut stream = iter(1_usize..=10).fuse();

			while let Some(result) = select![
				result: &mut stream => Some(result),
				r#yield => unreachable!(),
				complete => None,
			] {
				counter += 1;
				assert_eq!(counter, result);
			}

			assert_eq!(counter, 10);
		});
	}

	#[test]
	fn select_streams_fused() {
		block_on(async {
			let mut counter = 0;

			let mut stream1 = iter(1_usize..=10).fuse();
			let mut stream2 = iter(11..=20).fuse();

			while let Some(result) = select![
				result: &mut stream1 => Some(result),
				result: &mut stream2 => Some(result),
				r#yield => unreachable!(),
				complete => None,
			] {
				counter += 1;
				assert_eq!(counter, result);
			}

			assert_eq!(counter, 20);
		});
	}

	#[test]
	fn select_yield_1() {
		block_on(async {
			let mut counter = 0;
			let mut r#yield = 0_usize;

			let mut stream = {
				let mut iteration = 0_usize;
				let mut counter = 0_usize;

				poll_fn(move |_| {
					iteration += 1;

					match iteration {
						1..=10 => Poll::Pending,
						11..=20 => {
							counter += 1;
							Poll::Ready(Some(counter))
						}
						_ => Poll::Ready(None),
					}
				})
			}
			.fuse();

			while let Some(result) = select![
				result: &mut stream => Some(result),
				r#yield => r#yield += 1,
				complete => None,
			] {
				counter += 1;
				assert_eq!(counter, result);
			}

			assert_eq!(counter, 10);
			assert_eq!(r#yield, 10);
		});
	}

	#[test]
	fn select_yield_2() {
		block_on(async {
			let mut counter = 0;
			let mut r#yield = 0_usize;

			let mut stream1 = {
				let mut iteration = 0_usize;
				let mut counter = 10_usize;

				poll_fn(move |_| {
					iteration += 1;

					match iteration {
						1..=10 => Poll::Pending,
						11..=20 => {
							counter += 1;
							Poll::Ready(Some(counter))
						}
						_ => Poll::Ready(None),
					}
				})
			}
			.fuse();
			let mut stream2 = iter(1..=10).fuse();

			while let Some(result) = select![
				result: &mut stream1 => Some(result),
				result: &mut stream2 => Some(result),
				r#yield => r#yield += 1,
				complete => None,
			] {
				counter += 1;
				assert_eq!(counter, result);
			}

			assert_eq!(counter, 20);
			assert_eq!(r#yield, 0);
		});
	}
}
