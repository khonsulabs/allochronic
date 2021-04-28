use std::{
	array::IntoIter,
	collections::VecDeque,
	iter::FromIterator,
	pin::Pin,
	task::{Context, Poll},
};

use allochronic_channel::mpmc;
use allochronic_task::{LocalReceiver, LocalRunnable, Runnable};
use futures_util::{Stream, StreamExt};
use vec_map::VecMap;

type Receiver = mpmc::Receiver<Runnable>;

pub(crate) struct Priority<S: Stream>(VecMap<S>);

impl<S: Stream + Unpin> Stream for Priority<S> {
	type Item = S::Item;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let mut result = Poll::Ready(None);

		for receiver in self.0.values_mut() {
			match receiver.poll_next_unpin(cx) {
				Poll::Ready(Some(runnable)) => return Poll::Ready(Some(runnable)),
				Poll::Ready(None) => (),
				Poll::Pending => result = Poll::Pending,
			}
		}

		result
	}
}

impl Priority<Group<Queue, Queues>> {
	pub(super) fn new_queue(local: LocalReceiver, queue: Receiver) -> Self {
		Self(VecMap::from_iter(Some((
			0,
			Group(
				IntoIter::new([
					(Queue::Local, Queues::Local(local)),
					(Queue::Group(0), Queues::Group(queue)),
				])
				.collect(),
			),
		))))
	}
}

impl Priority<Group<Steal, Receiver>> {
	pub(super) fn new_stealer(injector: Receiver, stealer: Vec<Receiver>) -> Self {
		let injector = Some((Steal::Injector(0), injector)).into_iter();
		let stealer = stealer
			.into_iter()
			.map(|stealer| (Steal::Stealer(0), stealer));

		Self(VecMap::from_iter(Some((
			0,
			Group(injector.chain(stealer).collect()),
		))))
	}
}

impl<S: Stream<Item = Runnable>> Priority<S> {
	pub(super) fn extend<T: IntoIterator<Item = Receiver>>(
		&mut self,
		priority: usize,
		receiver: S,
	) {
		self.0.insert(priority, receiver);
	}

	pub(super) fn remove(&mut self, priority: usize) {
		self.0.remove(priority);
	}
}

impl<I: Copy + PartialEq + Unpin, S: Stream + Unpin> Priority<Group<I, S>> {
	pub(super) fn groups(&mut self, priority: usize) -> Option<&mut Group<I, S>> {
		self.0.get_mut(priority)
	}
}

pub(crate) struct Group<I: Copy + PartialEq + Unpin, S: Stream + Unpin>(VecDeque<(I, S)>);

impl<I: Copy + PartialEq + Unpin, S: Stream + Unpin> Stream for Group<I, S> {
	type Item = S::Item;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let group = self.get_mut();

		let mut result = Poll::Ready(None);
		let mut found = None;

		for (index, (_, ref mut receiver)) in group.0.iter_mut().enumerate() {
			match receiver.poll_next_unpin(cx) {
				Poll::Ready(Some(runnable)) => {
					result = Poll::Ready(Some(runnable));
					found = Some(index);
					break;
				}
				Poll::Ready(None) => (),
				Poll::Pending => result = Poll::Pending,
			}
		}

		if let Some(index) = found {
			let receiver = group.0.remove(index).expect("found receiver not found");
			group.0.push_back(receiver);
		}

		result
	}
}

impl<I: Copy + PartialEq + Unpin, S: Stream + Unpin> Group<I, S> {
	pub(super) fn new() -> Self {
		Self(VecDeque::default())
	}

	pub(super) fn extend<T: IntoIterator<Item = S>>(&mut self, id: I, iter: T) {
		self.0
			.extend(iter.into_iter().map(|receiver| (id, receiver)));
	}

	pub(super) fn remove(&mut self, id: I) {
		self.0.retain(|(index, _)| *index != id);
	}
}

pub(crate) enum Queues {
	Group(Receiver),
	Local(LocalReceiver),
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Queue {
	Group(usize),
	Local,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Steal {
	Injector(usize),
	Stealer(usize),
}

pub(crate) enum Runnables {
	Local(LocalRunnable),
	Group(Runnable),
}

impl Stream for Queues {
	type Item = Runnables;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		match self.get_mut() {
			Queues::Group(task) => task
				.poll_next_unpin(cx)
				.map(|option| option.map(Runnables::Group)),
			Queues::Local(task) => task
				.poll_next_unpin(cx)
				.map(|option| option.map(Runnables::Local)),
		}
	}
}

impl Runnables {
	pub(crate) fn run(self) {
		match self {
			Runnables::Local(task) => task.run(),
			Runnables::Group(task) => {
				task.run();
			}
		}
	}
}
