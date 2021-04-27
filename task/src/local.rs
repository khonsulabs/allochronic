use std::marker::PhantomData;

use async_task::Runnable;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct LocalRunnable(Runnable, PhantomData<*const ()>);

impl LocalRunnable {
	pub(crate) fn new(runnable: Runnable) -> Self {
		Self(runnable, PhantomData)
	}

	pub fn run(self) {
		self.0.run();
	}

	pub fn schedule(self) {
		self.0.schedule();
	}
}
