use std::{
	cell::RefCell,
	future::Future,
	panic,
	panic::AssertUnwindSafe,
	pin::Pin,
	process,
	rc::Rc,
	task::{Context, Poll},
};

use async_task::Task;
use futures_util::FutureExt;

use crate::{error, LocalRunnable, LocalSender};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct BlockedLocalTask<R>(Rc<RefCell<Option<Task<Finished<R>>>>>);

#[derive(Debug)]
pub struct Finished<R>(Inner<R>);

#[derive(Debug)]
enum Inner<R> {
	Output(R),
	Cancelled,
}

impl<R> Future for BlockedLocalTask<R> {
	type Output = Finished<R>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		#[allow(clippy::panic)]
		if let Some(task) = self.0.borrow_mut().as_mut() {
			task.poll_unpin(cx)
		} else {
			panic!("can't poll `Task` beyond completion");
		}
	}
}

impl<R> BlockedLocalTask<R> {
	pub fn cancel(self) -> Finished<R> {
		if let Some(task) = self.0.borrow_mut().take() {
			futures_lite::future::block_on(async move { task.cancel().await });
		}

		Finished(Inner::Cancelled)
	}
}

pub fn block_on_local<
	F: Future,
	M: FnOnce(LocalRunnable, BlockedLocalTask<F::Output>) -> Finished<F::Output>,
>(
	future: F,
	sender: LocalSender,
	main: M,
) -> Result<F::Output, error::Cancelled> {
	let (runnable, task) = unsafe {
		async_task::spawn_unchecked(
			async move { Finished(Inner::Output(future.await)) },
			move |runnable| {
				sender.send(LocalRunnable::new(runnable));
			},
		)
	};
	let runnable = LocalRunnable::new(runnable);
	let task = Rc::new(RefCell::new(Some(task)));
	let result = {
		let task = BlockedLocalTask(Rc::clone(&task));
		panic::catch_unwind(AssertUnwindSafe(move || main(runnable, task).0))
	};

	match result {
		Ok(result) => match result {
			Inner::Output(result) => Ok(result),
			Inner::Cancelled => Err(error::Cancelled),
		},
		Err(panic) => {
			if let Ok(mut task) = task.try_borrow_mut() {
				if let Some(task) = task.take() {
					futures_lite::future::block_on(task.cancel());
				}
			} else {
				process::abort()
			}

			panic::resume_unwind(panic)
		}
	}
}
