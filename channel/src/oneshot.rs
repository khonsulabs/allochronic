use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

use futures_channel::oneshot;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, Hash, Ord, PartialEq, PartialOrd)]
#[error("`Sender` was dropped")]
pub struct Canceled;

#[must_use]
pub fn oneshot<T>() -> (Sender<T>, Receiver<T>) {
	let (sender, receiver) = oneshot::channel();
	(Sender(sender), Receiver(receiver))
}

#[derive(Debug)]
pub struct Sender<T>(oneshot::Sender<T>);

#[derive(Debug)]
pub struct Receiver<T>(oneshot::Receiver<T>);

impl<T> Sender<T> {
	#[allow(clippy::expect_used)]
	pub fn send(self, data: T) {
		self.0
			.send(data)
			.map_err(|_data| ())
			.expect("`Receiver` dropped");
	}

	/// # Errors
	/// Returns `data` if the [`Receiver`] was dropped.
	pub fn try_send(self, data: T) -> Result<(), T> {
		self.0.send(data)
	}
}

impl<T> Future for Receiver<T>
where
	Self: Sized,
{
	type Output = Result<T, Canceled>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Pin::new(&mut self.as_mut().0)
			.poll(cx)
			.map_err(|_canceled| Canceled)
	}
}
