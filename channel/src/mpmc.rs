use std::{
	fmt::{self, Debug, Formatter},
	pin::Pin,
	task::{Context, Poll},
};

use flume::r#async::RecvStream;
use futures_util::{stream::Stream, StreamExt};

#[must_use]
pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
	let (sender, receiver) = flume::unbounded();

	(Sender(sender), Receiver(receiver.into_stream()))
}

#[derive(Debug)]
pub struct Sender<T>(flume::Sender<T>);

impl<T> Clone for Sender<T> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

pub struct Receiver<T: 'static>(RecvStream<'static, T>);

impl<T> Debug for Receiver<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_tuple("Receiver")
			.field(&String::from("RecvStream"))
			.finish()
	}
}

impl<T> Clone for Receiver<T> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<T> Sender<T> {
	pub fn send(&self, item: T) {
		#[allow(clippy::expect_used)]
		self.0.send(item).expect("no receiver alive");
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	#[must_use]
	pub fn len(&self) -> usize {
		self.0.len()
	}
}

impl<T> Receiver<T> {
	pub async fn clear(&mut self) {
		while let Poll::Ready(item) = allochronic_util::poll(self.next()).await {
			drop(item);
		}
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	#[must_use]
	pub fn len(&self) -> usize {
		self.0.len()
	}
}

impl<T> Stream for Receiver<T> {
	type Item = T;

	fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		Pin::new(&mut self.as_mut().0).poll_next(context)
	}
}
