use std::{
	future::Future,
	pin::Pin,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
	task::{Context, Poll},
};

use futures_util::task::AtomicWaker;

#[derive(Debug)]
pub struct Notify(Arc<Inner>);

#[derive(Clone, Debug)]
pub struct Notifier(Arc<Inner>);

#[derive(Debug)]
struct Inner {
	waker: AtomicWaker,
	status: AtomicBool,
}

impl Notify {
	#[must_use]
	pub fn new() -> Self {
		Self(Arc::new(Inner {
			waker: AtomicWaker::new(),
			status: AtomicBool::new(false),
		}))
	}

	#[must_use]
	pub fn register(&self) -> Notifier {
		Notifier(Arc::clone(&self.0))
	}

	pub fn notify(&self) {
		self.0.status.store(true, Ordering::SeqCst);
		self.0.waker.wake();
	}

	pub fn reset(&self) {
		self.0.status.store(false, Ordering::SeqCst);
	}
}

impl Default for Notify {
	fn default() -> Self {
		Self::new()
	}
}

impl Future for &Notify {
	type Output = ();

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self.0.waker.register(cx.waker());

		if self
			.0
			.status
			.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
			.is_ok()
		{
			Poll::Ready(())
		} else {
			Poll::Pending
		}
	}
}

impl Notifier {
	pub fn notify(&self) {
		self.0.status.store(true, Ordering::SeqCst);
		self.0.waker.wake();
	}
}
