#![deny(unsafe_code)]
#![warn(
	clippy::cargo,
	clippy::nursery,
	clippy::pedantic,
	clippy::restriction,
	future_incompatible,
	rust_2018_idioms
)]
#![warn(
	macro_use_extern_crate,
	meta_variable_misuse,
	missing_copy_implementations,
	missing_crate_level_docs,
	missing_debug_implementations,
	missing_docs,
	non_ascii_idents,
	single_use_lifetimes,
	trivial_casts,
	trivial_numeric_casts,
	unaligned_references,
	unreachable_pub,
	unused_import_braces,
	unused_lifetimes,
	unused_qualifications,
	variant_size_differences
)]
#![allow(
	clippy::blanket_clippy_restriction_lints,
	clippy::else_if_without_else,
	clippy::exhaustive_enums,
	clippy::exhaustive_structs,
	clippy::expect_used,
	clippy::future_not_send,
	clippy::implicit_return,
	clippy::missing_inline_in_public_items,
	clippy::non_ascii_literal,
	clippy::pattern_type_mismatch,
	clippy::redundant_pub_crate,
	clippy::shadow_reuse,
	clippy::tabs_in_doc_comments,
	clippy::unreachable,
	clippy::wildcard_enum_match_arm,
	unreachable_pub,
	variant_size_differences
)]
#![cfg_attr(
	doc,
	warn(rustdoc::all),
	allow(rustdoc::missing_doc_code_examples, rustdoc::private_doc_tests)
)]
// TODO: finish documentation
#![allow(
	clippy::missing_docs_in_private_items,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc,
	missing_docs
)]
#![allow(unsafe_code)]

mod channel;

use std::{
	cell::RefCell,
	future::Future,
	marker::PhantomData,
	panic::{self, AssertUnwindSafe},
	pin::Pin,
	process,
	rc::Rc,
	task::{Context, Poll},
};

pub use async_task::Runnable;
pub use channel::{unbounded, LocalReceiver, LocalSender};
use futures_util::FutureExt;
use thiserror::Error;

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

#[derive(Debug)]
pub struct BlockedLocalTask<R>(Rc<RefCell<Option<async_task::Task<Finished<R>>>>>);

#[derive(Debug)]
pub struct LocalRunnable(Runnable, PhantomData<*const ()>);

#[derive(Debug)]
pub struct Finished<R>(Inner<R>);

#[derive(Debug)]
enum Inner<R> {
	Output(R),
	Cancelled,
}

#[derive(Clone, Copy, Debug, Error)]
#[error("`Task` was cancelled")]
pub struct Cancelled;

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

impl LocalRunnable {
	pub fn run(self) {
		self.0.run();
	}

	pub fn schedule(self) {
		self.0.schedule();
	}
}

pub fn block_on_local<
	F: Future,
	M: FnOnce(LocalRunnable, BlockedLocalTask<F::Output>) -> Finished<F::Output>,
>(
	future: F,
	sender: LocalSender,
	main: M,
) -> Result<F::Output, Cancelled> {
	let (runnable, task) = unsafe {
		async_task::spawn_unchecked(
			async move { Finished(Inner::Output(future.await)) },
			move |runnable| {
				sender.send(LocalRunnable(runnable, PhantomData));
			},
		)
	};
	let runnable = LocalRunnable(runnable, PhantomData);
	let task = Rc::new(RefCell::new(Some(task)));
	let result = {
		let task = BlockedLocalTask(Rc::clone(&task));
		panic::catch_unwind(AssertUnwindSafe(move || main(runnable, task).0))
	};

	match result {
		Ok(result) => match result {
			Inner::Output(result) => Ok(result),
			Inner::Cancelled => Err(Cancelled),
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

pub fn spawn<F, S>(future: F, schedule: S) -> (Runnable, Task<F::Output>)
where
	F: Future + Send + 'static,
	F::Output: Send + 'static,
	S: Fn(Runnable) + Send + Sync + 'static,
{
	let (runnable, task) = async_task::spawn(future, schedule);

	(runnable, Task(Some(task)))
}
