use std::{
	future::Future,
	pin::Pin,
	sync::{atomic::Ordering, Arc},
	task::{Context, Poll},
};

use futures_util::FutureExt;
#[cfg(feature = "tokio-support")]
use tokio_util::context::TokioContext;

use crate::Worker;

#[derive(Debug)]
pub struct Task<R>(allochronic_task::Task<R>);

impl<R> Future for Task<R> {
	type Output = R;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self.0.poll_unpin(cx)
	}
}

impl<R> Task<R> {
	pub fn spawn<F>(future: F) -> Self
	where
		F: Future<Output = R> + Send + 'static,
		R: Send + 'static,
	{
		let executor = Worker::with(move |worker| Arc::clone(&worker.executor));
		executor.tasks.fetch_add(1, Ordering::SeqCst);
		#[cfg(feature = "tokio-support")]
		let future = TokioContext::new(future, executor.tokio.handle().clone());

		let (runnable, task) = allochronic_task::spawn(
			async move {
				let result = future.await;
				Worker::with(|worker| {
					worker.executor.tasks.fetch_sub(1, Ordering::Relaxed);
				});
				result
			},
			move |runnable| {
				Worker::try_with(|worker| {
					if let Some(worker) = worker {
						worker
							.injector
							.get(0)
							.and_then(|group| group.get(0))
							.expect("default group not found")
							.send(runnable);
					} else {
						executor
							.injector
							.read()
							.get(0)
							.and_then(|group| group.get(0))
							.expect("default group not found")
							.0
							.send(runnable);
					}
				});
			},
		);
		runnable.schedule();

		Self(task)
	}
}
