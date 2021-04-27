mod queues;

use std::{
	cell::{Ref, RefCell, RefMut},
	future::Future,
	iter::FromIterator,
	sync::{atomic::AtomicUsize, Arc},
};

use allochronic_channel::{broadcast, flag::Flag, mpmc};
use allochronic_task::{LocalReceiver, LocalSender, Runnable};
use core_affinity::CoreId;
use once_cell::unsync::OnceCell;
use queues::{Group, Priority, Queue, Queues, RunnableWrapper, Steal};
use vec_map::VecMap;

use crate::{error, Executor};

type Sender = mpmc::Sender<Runnable>;
type Receiver = mpmc::Receiver<Runnable>;

pub(crate) struct Worker {
	pub(crate) executor: Arc<Executor>,
	type_: Type,
	pub(crate) injector: VecMap<VecMap<Sender>>,
	local: VecMap<LocalSender>,
	inner: Inner,
}

enum Type {
	Async {
		queue: Priority<Group<Queue, Queues>>,
		stealer: Priority<Group<Steal, Receiver>>,
	},
	Blocking {
		count: AtomicUsize,
		local: Priority<LocalReceiver>,
	},
}

struct Inner {
	shutdown: Flag,
	management: broadcast::Receiver<()>,
}

enum Message<R> {
	Shutdown,
	Management(()),
	Main(R),
	Task(RunnableWrapper),
}

// TODO: fix Clippy
#[allow(clippy::use_self)]
impl Worker {
	thread_local!(pub (crate) static WORKER: OnceCell<RefCell<Worker>> = OnceCell::new());
}

impl Worker {
	pub(crate) fn with<F: FnOnce(Ref<'_, Self>) -> R, R>(fun: F) -> R {
		Self::WORKER.with(|worker| fun(worker.get().expect("`Worker` not initialized").borrow()))
	}

	pub(crate) fn try_with<F: FnOnce(Option<Ref<'_, Self>>) -> R, R>(fun: F) -> R {
		Self::WORKER.with(|worker| fun(worker.get().map(RefCell::borrow)))
	}

	fn with_mut<F: FnOnce(RefMut<'_, Self>) -> R, R>(fun: F) -> R {
		Self::WORKER
			.with(|worker| fun(worker.get().expect("`Worker` not initialized").borrow_mut()))
	}

	fn init(
		executor: Arc<Executor>,
		core: Option<CoreId>,
		sender: Sender,
		receiver: Receiver,
		injector: Receiver,
		stealer: Vec<Receiver>,
	) {
		if let Some(core) = core {
			core_affinity::set_for_current(core);
		}

		let (local_sender, local_receiver) = allochronic_task::unbounded();

		let type_ = Type::Async {
			queue: Priority::new_queue(local_receiver, receiver),
			stealer: Priority::new_stealer(injector, stealer),
		};

		let inner = Inner {
			shutdown: executor.shutdown.clone(),
			management: executor.management(),
		};

		Self::WORKER
			.with(|worker| {
				worker.set(RefCell::new(Self {
					executor,
					type_,
					injector: VecMap::from_iter(Some((0, VecMap::from_iter(Some((0, sender)))))),
					local: VecMap::from_iter(Some((0, local_sender))),
					inner,
				}))
			})
			.map_err(|_old| ())
			.expect("`Worker` can't be initialized twice");
	}

	fn run<'w, S, F, R>(worker: &'w mut Self, select: S) -> Message<R>
	where
		S: FnOnce(
			&'w mut Flag,
			&'w mut broadcast::Receiver<()>,
			&'w mut Priority<Group<Queue, Queues>>,
			&'w mut Priority<Group<Steal, Receiver>>,
		) -> F,
		F: Future<Output = Option<Message<R>>>,
	{
		#[allow(clippy::await_holding_refcell_ref)]
		futures_lite::future::block_on(async move {
			let Worker {
				inner: Inner {
					shutdown,
					management,
				},
				type_,
				..
			} = worker;
			let (stealer, queue) = if let Type::Async { stealer, queue } = type_ {
				(stealer, queue)
			} else {
				unreachable!("`Worker` is not async")
			};

			if let Some(message) = select(shutdown, management, queue, stealer).await {
				message
			} else {
				unreachable!("a `Sender` dropped");
			}
		})
	}

	pub(crate) fn start(
		executor: Arc<Executor>,
		core: Option<CoreId>,
		sender: Sender,
		receiver: Receiver,
		injector: Receiver,
		stealer: Vec<Receiver>,
	) {
		Self::init(executor, core, sender, receiver, injector, stealer);

		Self::WORKER.with(|worker| {
			let worker = worker.get().expect("`Worker` not initialized");

			loop {
				let message = Self::run(
					&mut *worker.borrow_mut(),
					|shutdown, management, queue, stealer| async move {
						allochronic_util::select!(
							_: shutdown => Some(Message::Shutdown),
							management: management => management.map(Message::Management),
							task: queue => task.map(Message::Task),
							task: stealer => task.map(RunnableWrapper::Group).map(Message::Task),
						)
					},
				);

				match message {
					Message::Main(()) => unreachable!("returned `main` in wrong function"),
					Message::Shutdown => break,
					Message::Management(()) => (),
					Message::Task(runnable) => {
						runnable.run();
					}
				}
			}
		});
	}

	pub(crate) fn start_with<M, R>(
		executor: Arc<Executor>,
		core: Option<CoreId>,
		sender: Sender,
		receiver: Receiver,
		injector: Receiver,
		stealer: Vec<Receiver>,
		main: M,
	) -> Result<R, error::Executor>
	where
		M: Future<Output = R>,
	{
		Self::init(executor, core, sender, receiver, injector, stealer);

		Self::WORKER
			.with(|worker| {
				let worker = worker.get().expect("`Worker` not initialized");
				allochronic_task::block_on_local(
					main,
					worker
						.borrow()
						.local
						.get(0)
						.expect("initial group doesn't exist")
						.clone(),
					|runnable, mut task| {
						runnable.schedule();

						loop {
							let message = Self::run(
								&mut *worker.borrow_mut(),
								|shutdown, management, queue, stealer| {
									let task = &mut task;
									async move {
										allochronic_util::select!(
											result: task => Some(Message::Main(result)),
											_: shutdown => Some(Message::Shutdown),
											management: management => {
												management.map(Message::Management)
											},
											task: queue => task.map(Message::Task),
											task: stealer => {
												task.map(RunnableWrapper::Group).map(Message::Task)
											},
										)
									}
								},
							);

							match message {
								Message::Main(result) => break result,
								Message::Shutdown => break task.cancel(),
								Message::Management(()) => (),
								Message::Task(runnable) => {
									runnable.run();
								}
							}
						}
					},
				)
			})
			.map_err(|_cancelled| error::Executor::Cancelled)
	}
}
