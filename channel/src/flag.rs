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
struct Inner {
	waker: AtomicWaker,
	set: AtomicBool,
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Clone, Debug)]
pub struct Flag(Arc<Inner>);

impl Flag {
	pub fn new() -> Self {
		Self(Arc::new(Inner {
			waker: AtomicWaker::new(),
			set: AtomicBool::new(false),
		}))
	}

	pub fn signal(&self) {
		self.0.set.store(true, Ordering::Relaxed);
		self.0.waker.wake();
	}
}

impl Default for Flag {
	fn default() -> Self {
		Self::new()
	}
}

impl Future for Flag {
	type Output = ();

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
		if self.0.set.load(Ordering::Relaxed) {
			return Poll::Ready(());
		}

		self.0.waker.register(cx.waker());

		if self.0.set.load(Ordering::Relaxed) {
			Poll::Ready(())
		} else {
			Poll::Pending
		}
	}
}
