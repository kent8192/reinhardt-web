//! Cooperative cancellation primitives used by route-loader preparation.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll, Waker};

use futures_util::future::{AbortHandle, Abortable};

use crate::platform::spawn_task;

thread_local! {
	static ACTIVE_CANCELLATION: RefCell<Option<CancellationHandle>> = const { RefCell::new(None) };
}

/// Extractor wrapper supplied to a route loader when it requests cancellation.
///
/// The tuple form intentionally lets a loader name the inner handle directly:
/// `CancellationToken(cancel): CancellationToken`.
#[derive(Clone)]
pub struct CancellationToken(pub CancellationHandle);

/// A cloneable cooperative cancellation handle.
#[derive(Clone)]
pub struct CancellationHandle {
	inner: Rc<CancellationState>,
}

struct CancellationState {
	cancelled: Cell<bool>,
	next_registration: Cell<u64>,
	callbacks: RefCell<BTreeMap<u64, Rc<dyn Fn()>>>,
	wakers: RefCell<BTreeMap<u64, Waker>>,
}

/// Error returned when a cancellation check observes a cancelled operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.write_str("operation cancelled")
	}
}

impl std::error::Error for Cancelled {}

impl CancellationHandle {
	/// Returns whether cancellation has been requested.
	pub fn is_cancelled(&self) -> bool {
		self.inner.cancelled.get()
	}

	/// Returns [`Cancelled`] when cancellation has been requested.
	pub fn check(&self) -> Result<(), Cancelled> {
		if self.is_cancelled() {
			Err(Cancelled)
		} else {
			Ok(())
		}
	}

	/// Returns a future that resolves when cancellation is requested.
	pub fn cancelled(&self) -> CancellationFuture {
		CancellationFuture {
			inner: Rc::clone(&self.inner),
			waiter_id: None,
		}
	}

	pub(crate) fn on_cancel(&self, callback: impl Fn() + 'static) -> CancellationRegistration {
		register_callback(&self.inner, callback)
	}
}

/// Future returned by [`CancellationHandle::cancelled`].
pub struct CancellationFuture {
	inner: Rc<CancellationState>,
	waiter_id: Option<u64>,
}

impl Future for CancellationFuture {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
		if self.inner.cancelled.get() {
			self.waiter_id = None;
			return Poll::Ready(());
		}

		let waiter_id = if let Some(waiter_id) = self.waiter_id {
			waiter_id
		} else {
			let waiter_id = next_id(&self.inner);
			self.inner
				.wakers
				.borrow_mut()
				.insert(waiter_id, context.waker().clone());
			self.waiter_id = Some(waiter_id);
			waiter_id
		};
		let mut wakers = self.inner.wakers.borrow_mut();
		if let Some(previous) = wakers.get_mut(&waiter_id)
			&& !previous.will_wake(context.waker())
		{
			*previous = context.waker().clone();
		}
		Poll::Pending
	}
}

impl Drop for CancellationFuture {
	fn drop(&mut self) {
		if let Some(waiter_id) = self.waiter_id.take() {
			self.inner.wakers.borrow_mut().remove(&waiter_id);
		}
	}
}

/// Source used by a coordinator attempt to request cancellation.
pub(crate) struct CancellationSource {
	inner: Rc<CancellationState>,
}

impl CancellationSource {
	pub(crate) fn new() -> Self {
		Self {
			inner: Rc::new(CancellationState {
				cancelled: Cell::new(false),
				next_registration: Cell::new(0),
				callbacks: RefCell::new(BTreeMap::new()),
				wakers: RefCell::new(BTreeMap::new()),
			}),
		}
	}

	pub(crate) fn handle(&self) -> CancellationHandle {
		CancellationHandle {
			inner: Rc::clone(&self.inner),
		}
	}

	pub(crate) fn cancel(&self) {
		if self.inner.cancelled.replace(true) {
			return;
		}

		let callbacks = std::mem::take(&mut *self.inner.callbacks.borrow_mut())
			.into_values()
			.collect::<Vec<_>>();
		let wakers = std::mem::take(&mut *self.inner.wakers.borrow_mut())
			.into_values()
			.collect::<Vec<_>>();

		for callback in callbacks {
			callback();
		}
		for waker in wakers {
			waker.wake();
		}
	}

	pub(crate) fn register(&self, callback: impl Fn() + 'static) -> CancellationRegistration {
		register_callback(&self.inner, callback)
	}
}

fn register_callback(
	inner: &Rc<CancellationState>,
	callback: impl Fn() + 'static,
) -> CancellationRegistration {
	if inner.cancelled.get() {
		callback();
		return CancellationRegistration {
			state: Weak::new(),
			id: None,
		};
	}

	let id = next_id(inner);
	inner.callbacks.borrow_mut().insert(id, Rc::new(callback));
	CancellationRegistration {
		state: Rc::downgrade(inner),
		id: Some(id),
	}
}

impl Drop for CancellationSource {
	fn drop(&mut self) {
		self.cancel();
	}
}

/// RAII registration for a cancellation callback.
pub(crate) struct CancellationRegistration {
	state: Weak<CancellationState>,
	id: Option<u64>,
}

impl Drop for CancellationRegistration {
	fn drop(&mut self) {
		if let Some(id) = self.id.take()
			&& let Some(state) = self.state.upgrade()
		{
			state.callbacks.borrow_mut().remove(&id);
		}
	}
}

fn next_id(state: &Rc<CancellationState>) -> u64 {
	let id = state.next_registration.get();
	state.next_registration.set(id.wrapping_add(1));
	id
}

/// Installs a cancellation token for each poll of a future.
pub(crate) struct ScopedCancellation<F> {
	token: CancellationHandle,
	future: Pin<Box<F>>,
}

struct ActiveCancellationGuard {
	previous: Option<CancellationHandle>,
}

impl Drop for ActiveCancellationGuard {
	fn drop(&mut self) {
		let previous = self.previous.take();
		ACTIVE_CANCELLATION.with(|slot| {
			*slot.borrow_mut() = previous;
		});
	}
}

impl<F> Future for ScopedCancellation<F>
where
	F: Future,
{
	type Output = F::Output;

	fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
		let previous =
			ACTIVE_CANCELLATION.with(|slot| slot.borrow_mut().replace(self.token.clone()));
		let _guard = ActiveCancellationGuard { previous };
		self.future.as_mut().poll(context)
	}
}

pub(crate) fn scope_cancellation<F>(token: CancellationHandle, future: F) -> ScopedCancellation<F> {
	ScopedCancellation {
		token,
		future: Box::pin(future),
	}
}

pub(crate) fn active_cancellation() -> Option<CancellationHandle> {
	ACTIVE_CANCELLATION.with(|slot| slot.borrow().clone())
}

/// A task guard that aborts its associated future when dropped.
pub(crate) struct AbortableTaskGuard {
	handle: Option<AbortHandle>,
}

impl AbortableTaskGuard {
	pub(crate) fn new(handle: AbortHandle) -> Self {
		Self {
			handle: Some(handle),
		}
	}
}

impl Drop for AbortableTaskGuard {
	fn drop(&mut self) {
		if let Some(handle) = self.handle.take() {
			handle.abort();
		}
	}
}

pub(crate) fn spawn_abortable_task<F>(future: F) -> AbortableTaskGuard
where
	F: Future<Output = ()> + 'static,
{
	let (handle, registration) = AbortHandle::new_pair();
	spawn_task(async move {
		let _ = Abortable::new(future, registration).await;
	});
	AbortableTaskGuard::new(handle)
}

#[cfg(test)]
mod tests {
	use std::cell::{Cell, RefCell};
	use std::future::{Future, poll_fn};
	use std::rc::Rc;
	use std::sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	};
	use std::task::{Context, Poll, Wake, Waker};

	use futures_util::future::{AbortHandle, Abortable};

	use super::*;

	struct CountingWake(Arc<AtomicUsize>);

	impl Wake for CountingWake {
		fn wake(self: Arc<Self>) {
			self.0.fetch_add(1, Ordering::SeqCst);
		}

		fn wake_by_ref(self: &Arc<Self>) {
			self.0.fetch_add(1, Ordering::SeqCst);
		}
	}

	fn counting_waker(counter: Arc<AtomicUsize>) -> Waker {
		Waker::from(Arc::new(CountingWake(counter)))
	}

	fn handles_match(left: &CancellationHandle, right: &CancellationHandle) -> bool {
		Rc::ptr_eq(&left.inner, &right.inner)
	}

	#[test]
	fn cancellation_source_is_idempotent() {
		let source = CancellationSource::new();
		let handle = source.handle();
		let callbacks = Rc::new(Cell::new(0));
		let callbacks_for_registration = Rc::clone(&callbacks);
		let _registration = source.register(move || {
			callbacks_for_registration.set(callbacks_for_registration.get() + 1);
		});

		assert!(!handle.is_cancelled());
		assert_eq!(handle.check(), Ok(()));

		source.cancel();
		source.cancel();

		assert!(handle.is_cancelled());
		assert_eq!(handle.check(), Err(Cancelled));
		assert_eq!(callbacks.get(), 1);
	}

	#[test]
	fn dropping_the_first_registration_removes_its_callback() {
		let source = CancellationSource::new();
		let callbacks = Rc::new(Cell::new(0));
		let callbacks_for_registration = Rc::clone(&callbacks);
		let registration = source.register(move || {
			callbacks_for_registration.set(callbacks_for_registration.get() + 1);
		});

		drop(registration);
		source.cancel();

		assert_eq!(callbacks.get(), 0);
	}

	#[test]
	fn cancelled_future_wakes_once() {
		let source = CancellationSource::new();
		let mut future = Box::pin(source.handle().cancelled());
		let wake_count = Arc::new(AtomicUsize::new(0));
		let waker = counting_waker(Arc::clone(&wake_count));
		let mut context = Context::from_waker(&waker);

		assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));

		source.cancel();
		source.cancel();

		assert_eq!(wake_count.load(Ordering::SeqCst), 1);
		assert!(matches!(
			future.as_mut().poll(&mut context),
			Poll::Ready(())
		));
	}

	#[test]
	fn scoped_token_is_restored_after_pending_poll() {
		let source = CancellationSource::new();
		let token = source.handle();
		let token_for_future = token.clone();
		let polled = Rc::new(Cell::new(false));
		let polled_for_future = Rc::clone(&polled);
		let future = poll_fn(move |_| {
			assert!(
				active_cancellation()
					.as_ref()
					.is_some_and(|active| handles_match(active, &token_for_future))
			);
			polled_for_future.set(true);
			Poll::<()>::Pending
		});
		let mut scoped = Box::pin(scope_cancellation(token, future));
		let mut context = Context::from_waker(Waker::noop());

		assert!(!polled.get());
		assert!(matches!(scoped.as_mut().poll(&mut context), Poll::Pending));
		assert!(polled.get());
		assert!(active_cancellation().is_none());
	}

	#[test]
	fn sibling_scoped_futures_do_not_mix_tokens() {
		let first_source = CancellationSource::new();
		let second_source = CancellationSource::new();
		let first_token = first_source.handle();
		let second_token = second_source.handle();
		let first_seen = Rc::new(RefCell::new(Vec::new()));
		let second_seen = Rc::new(RefCell::new(Vec::new()));

		let first_for_future = first_token.clone();
		let first_seen_for_future = Rc::clone(&first_seen);
		let first_future = poll_fn(move |_| {
			first_seen_for_future.borrow_mut().push(
				active_cancellation()
					.is_some_and(|active| handles_match(&active, &first_for_future)),
			);
			Poll::<()>::Pending
		});
		let second_for_future = second_token.clone();
		let second_seen_for_future = Rc::clone(&second_seen);
		let second_future = poll_fn(move |_| {
			second_seen_for_future.borrow_mut().push(
				active_cancellation()
					.is_some_and(|active| handles_match(&active, &second_for_future)),
			);
			Poll::<()>::Pending
		});

		let mut first = Box::pin(scope_cancellation(first_token, first_future));
		let mut second = Box::pin(scope_cancellation(second_token, second_future));
		let mut context = Context::from_waker(Waker::noop());

		assert!(matches!(first.as_mut().poll(&mut context), Poll::Pending));
		assert!(active_cancellation().is_none());
		assert!(matches!(second.as_mut().poll(&mut context), Poll::Pending));
		assert!(active_cancellation().is_none());
		assert_eq!(&*first_seen.borrow(), &[true]);
		assert_eq!(&*second_seen.borrow(), &[true]);
	}

	#[test]
	fn dropping_abortable_task_guard_stops_polling() {
		let polls = Rc::new(Cell::new(0));
		let polls_for_future = Rc::clone(&polls);
		let future = poll_fn(move |_| {
			polls_for_future.set(polls_for_future.get() + 1);
			Poll::<()>::Pending
		});
		let (abort_handle, abort_registration) = AbortHandle::new_pair();
		let mut abortable = Box::pin(Abortable::new(future, abort_registration));
		let guard = AbortableTaskGuard::new(abort_handle);
		let mut context = Context::from_waker(Waker::noop());

		assert!(matches!(
			abortable.as_mut().poll(&mut context),
			Poll::Pending
		));
		drop(guard);
		assert!(matches!(
			abortable.as_mut().poll(&mut context),
			Poll::Ready(Err(_))
		));
		assert_eq!(polls.get(), 1);
	}
}
