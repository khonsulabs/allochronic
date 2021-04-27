use std::{
	future::Future,
	iter::FromIterator,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
	thread,
};

use allochronic_channel::{broadcast, flag::Flag, mpmc, notify::Notify, oneshot};
use allochronic_task::Runnable;
use parking_lot::{Mutex, RwLock};
#[cfg(feature = "tokio-support")]
use tokio::runtime::Runtime;
use vec_map::VecMap;

use crate::{error, Worker};

type Sender = mpmc::Sender<Runnable>;
type Receiver = mpmc::Receiver<Runnable>;

#[derive(Debug)]
pub struct Executor {
	pub(crate) tasks: AtomicUsize,
	threads: Mutex<Vec<oneshot::Receiver<thread::Result<()>>>>,
	pub(crate) shutdown: Flag,
	pub(crate) finished: Notify,
	management: broadcast::Sender<()>,
	pub(crate) injector: RwLock<VecMap<VecMap<(Sender, Receiver)>>>,
	#[cfg(feature = "tokio-support")]
	pub(crate) tokio: Runtime,
}

impl Executor {
	pub fn start<M, R>(main: M) -> R
	where
		M: Future<Output = R>,
	{
		Self::try_start(main).expect("`Executor::start` failed")
	}

	#[allow(clippy::panic_in_result_fn, clippy::unwrap_in_result)]
	pub fn try_start<M, R>(main: M) -> Result<R, error::Executor>
	where
		M: Future<Output = R>,
	{
		let cores = match core_affinity::get_core_ids() {
			Some(cores) if !cores.is_empty() => cores.into_iter().map(Some).collect(),
			_ => {
				let cores = match num_cpus::get_physical() {
					0 => unreachable!("no cores found"),
					cores => cores,
				};
				vec![None; cores]
			}
		};

		let injector = mpmc::unbounded();
		let injector_receiver = injector.1.clone();

		// add tokio support
		#[cfg(feature = "tokio-support")]
		let tokio = tokio::runtime::Builder::new_multi_thread()
			.enable_all()
			.build()
			.expect("failed to build tokio `Runtime`");

		let executor = Arc::new(Self {
			tasks: AtomicUsize::new(0),
			threads: Mutex::default(),
			shutdown: Flag::new(),
			finished: Notify::new(),
			management: broadcast::unbounded(),
			injector: RwLock::new(VecMap::from_iter(Some((
				0,
				VecMap::from_iter(Some((0, injector))),
			)))),
			#[cfg(feature = "tokio-support")]
			tokio,
		});

		let threads: Vec<_> = {
			let queues: Vec<_> = (0..cores.len()).map(|_| mpmc::unbounded()).collect();

			cores
				.into_iter()
				.zip(queues.clone())
				.enumerate()
				.map(|(index, (core, (sender, receiver)))| {
					let stealer: Vec<_> = queues
						.iter()
						.enumerate()
						.filter(|(queue, _)| *queue != index)
						.map(|(_, (_, receiver))| receiver.clone())
						.collect();

					(core, sender, receiver, stealer)
				})
				.collect()
		};

		let mut threads = threads.into_iter();
		let main_thread = threads.next().expect("no main thread found");

		for (core, sender, receiver, stealer) in threads {
			let handle = {
				let executor = Arc::clone(&executor);
				let injector = injector_receiver.clone();

				thread::spawn(move || {
					Worker::start(executor, core, sender, receiver, injector, stealer);
				})
			};

			let (sender, receiver) = oneshot::oneshot();
			executor.threads.lock().push(receiver);

			thread::spawn(move || {
				sender.send(handle.join());
			});
		}

		let (core, sender, receiver, stealer) = main_thread;

		Worker::start_with(
			executor,
			core,
			sender,
			receiver,
			injector_receiver,
			stealer,
			main,
		)
	}

	pub(crate) fn management(&self) -> broadcast::Receiver<()> {
		self.management.subscribe()
	}

	pub async fn wait() {
		let executor = Worker::with(|worker| Arc::clone(&worker.executor));

		loop {
			if executor.tasks.load(Ordering::SeqCst) == 0 {
				break;
			}

			(&executor.finished).await;
		}
	}
}
