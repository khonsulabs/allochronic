use std::{
	future::Future,
	pin::Pin,
	sync::{atomic::Ordering, Arc},
	task::{Context, Poll},
};

use allochronic_task::Runnable;
use futures_util::FutureExt;
#[cfg(feature = "tokio-support")]
use tokio_util::context::TokioContext;

use crate::{Executor, Message, Runnables, Worker};

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
			move |runnable| Self::send_injector(&executor, runnable),
		);
		runnable.schedule();

		Self(task)
	}

	pub fn block_on<F>(future: F) -> R
	where
		F: Future<Output = R> + Send,
		R: Send,
	{
		Worker::WORKER.with(move |worker| {
			let worker = worker.get().expect("`Worker` not initialized");

			let executor = Arc::clone(&worker.borrow().executor);
			executor.tasks.fetch_add(1, Ordering::SeqCst);
			#[cfg(feature = "tokio-support")]
			let future = TokioContext::new(future, executor.tokio.handle().clone());

			allochronic_task::block_on(
				async move {
					let result = future.await;
					Worker::with(|worker| {
						worker.executor.tasks.fetch_sub(1, Ordering::Relaxed);
					});
					result
				},
				move |runnable| Self::send_injector(&executor, runnable),
				|runnable, mut task| {
					runnable.schedule();

					loop {
						let message = {
							let worker = &mut *worker.borrow_mut();

							Worker::run(worker, |shutdown, management, queue, stealer| {
								let task = &mut task;
								async move {
									allochronic_util::select!(
										_: shutdown => Some(Message::Shutdown),
										management: management => {
											management.map(Message::Management)
										},
										result: task => Some(Message::Blocked(result)),
										task: queue => task.map(Message::Task),
										task: stealer => {
											task.map(Runnables::Group).map(Message::Task)
										},
									)
								}
							})
						};

						match message {
							Message::Blocked(result) => break result,
							Message::Shutdown => break task.cancel(),
							Message::Management(()) => (),
							Message::Task(runnable) => {
								runnable.run();
							}
						}

						{
							let executor = &worker.borrow().executor;

							if executor.tasks.load(Ordering::Relaxed) == 0 {
								executor.finished.notify();
							}
						}
					}
				},
			)
			.expect("`Task` cancelled, likely because of `Executor` shutdown")
		})
	}

	fn send_injector(executor: &Arc<Executor>, runnable: Runnable) {
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
	}
}
