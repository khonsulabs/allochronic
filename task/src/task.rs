use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

use async_task::Runnable;
use futures_util::FutureExt;

#[derive(Debug)]
pub struct Task<R>(Option<async_task::Task<R>>);

impl<R> Future for Task<R> {
	type Output = R;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		if let Some(task) = self.0.as_mut() {
			task.poll_unpin(cx)
		} else {
			unreachable!("`Task` polled after drop")
		}
	}
}

impl<R> Drop for Task<R> {
	fn drop(&mut self) {
		self.0.take().expect("`Task` already dropped").detach();
	}
}

pub fn spawn<F, S>(future: F, schedule: S) -> (Runnable, Task<F::Output>)
where
	F: Future + Send + 'static,
	F::Output: Send + 'static,
	S: Fn(Runnable) + Send + Sync + 'static,
{
	let (runnable, task) = async_task::spawn(future, schedule);

	(runnable, Task(Some(task)))
}
