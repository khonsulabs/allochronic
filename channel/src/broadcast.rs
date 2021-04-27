//! [`Broadcasting`](self) channel implementation.
// TODO: this will need an overhaul if alternatives to `flume` can be found

use std::{
	fmt::{self, Debug, Formatter},
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
};

use flume::r#async::RecvStream;
use futures_util::stream::Stream;
use parking_lot::RwLock;

#[must_use]
pub fn unbounded<T: Clone>() -> Sender<T> {
	Sender(Arc::default())
}

#[derive(Debug)]
pub struct Sender<T: Clone>(Arc<RwLock<Vec<flume::Sender<T>>>>);

pub struct Receiver<T: 'static>(RecvStream<'static, T>);

impl<T> Debug for Receiver<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_tuple("Receiver")
			.field(&String::from("RecvStream"))
			.finish()
	}
}

impl<T: Clone> Sender<T> {
	#[must_use]
	pub fn subscribe(&self) -> Receiver<T> {
		let (sender, receiver) = flume::unbounded();
		// locking is only done for a very short period of time
		// making async-locking probably not worth the cost
		self.0.write().push(sender);
		Receiver(receiver.into_stream())
	}

	pub fn send(&self, message: T) {
		// throw out any `Sender`s in the process of sending that don't have a receiver
		// anymore
		self.0
			.write()
			.retain(|sender| sender.send(message.clone()).is_ok());
	}
}

impl<T> Stream for Receiver<T> {
	type Item = T;

	fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		Pin::new(&mut self.as_mut().0).poll_next(context)
	}
}
