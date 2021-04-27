use std::{
	marker::PhantomData,
	pin::Pin,
	task::{Context, Poll},
};

use allochronic_channel::mpmc;
pub use async_task::Runnable;
use futures_util::{Stream, StreamExt};
use mpmc::{Receiver, Sender};

use crate::LocalRunnable;

#[derive(Clone, Debug)]
pub struct LocalSender(Sender<LocalRunnable>, PhantomData<*const ()>);

#[derive(Clone, Debug)]
pub struct LocalReceiver(Receiver<LocalRunnable>, PhantomData<*const ()>);

impl Stream for LocalReceiver {
	type Item = LocalRunnable;

	fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.0.poll_next_unpin(context)
	}
}

impl LocalSender {
	pub fn send(&self, item: LocalRunnable) {
		self.0.send(item);
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

impl LocalReceiver {
	pub async fn clear(&mut self) {
		self.0.clear().await;
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

#[must_use]
pub fn unbounded() -> (LocalSender, LocalReceiver) {
	let (sender, receiver) = mpmc::unbounded();

	(
		LocalSender(sender, PhantomData),
		LocalReceiver(receiver, PhantomData),
	)
}
