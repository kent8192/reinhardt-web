//! Target-aware state containers for MSW runtimes.

use super::handler::ErasedHandler;
use super::recorder::{RecordedRequest, RequestRecorder};

#[cfg(wasm)]
use std::cell::{Cell, Ref, RefCell};
#[cfg(wasm)]
use std::rc::Rc;

#[cfg(native)]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[cfg(native)]
use std::sync::{Arc, Mutex, MutexGuard};

#[cfg(native)]
pub(crate) trait HandlerThreadSafety: Send + Sync {}
#[cfg(native)]
impl<T: Send + Sync> HandlerThreadSafety for T {}

#[cfg(wasm)]
pub(crate) trait HandlerThreadSafety {}
#[cfg(wasm)]
impl<T> HandlerThreadSafety for T {}

#[cfg(native)]
pub(crate) type ResponseFn =
	dyn Fn(&super::handler::InterceptedRequest) -> super::response::MockResponse + Send + Sync;

#[cfg(wasm)]
pub(crate) type ResponseFn =
	dyn Fn(&super::handler::InterceptedRequest) -> super::response::MockResponse;

/// A request predicate registered with [`RestHandlerBuilder::matching`](super::rest::RestHandlerBuilder::matching).
#[cfg(native)]
pub(crate) type PredicateFn = dyn Fn(&super::handler::InterceptedRequest) -> bool + Send + Sync;

#[cfg(wasm)]
pub(crate) type PredicateFn = dyn Fn(&super::handler::InterceptedRequest) -> bool;

/// Counts the requests a handler answered, for call expectations.
#[cfg(wasm)]
pub(crate) struct HitCounter(Cell<usize>);

#[cfg(wasm)]
impl HitCounter {
	pub(crate) fn new() -> Self {
		Self(Cell::new(0))
	}

	pub(crate) fn increment(&self) {
		self.0.set(self.0.get() + 1);
	}

	pub(crate) fn get(&self) -> usize {
		self.0.get()
	}
}

#[cfg(native)]
pub(crate) struct HitCounter(AtomicUsize);

#[cfg(native)]
impl HitCounter {
	pub(crate) fn new() -> Self {
		Self(AtomicUsize::new(0))
	}

	pub(crate) fn increment(&self) {
		self.0.fetch_add(1, Ordering::SeqCst);
	}

	pub(crate) fn get(&self) -> usize {
		self.0.load(Ordering::SeqCst)
	}
}

#[cfg(wasm)]
pub(crate) struct OnceFlag(Cell<bool>);

#[cfg(wasm)]
impl OnceFlag {
	pub(crate) fn new(value: bool) -> Self {
		Self(Cell::new(value))
	}

	pub(crate) fn get(&self) -> bool {
		self.0.get()
	}

	pub(crate) fn set(&self, value: bool) {
		self.0.set(value);
	}
}

#[cfg(native)]
pub(crate) struct OnceFlag(AtomicBool);

#[cfg(native)]
impl OnceFlag {
	pub(crate) fn new(value: bool) -> Self {
		Self(AtomicBool::new(value))
	}

	pub(crate) fn get(&self) -> bool {
		self.0.load(Ordering::SeqCst)
	}

	pub(crate) fn set(&self, value: bool) {
		self.0.store(value, Ordering::SeqCst);
	}
}

#[cfg(wasm)]
#[derive(Clone)]
pub(crate) struct SharedHandlers(Rc<RefCell<Vec<Box<dyn ErasedHandler>>>>);

#[cfg(wasm)]
impl SharedHandlers {
	pub(crate) fn new() -> Self {
		Self(Rc::new(RefCell::new(Vec::new())))
	}

	pub(crate) fn push(&self, handler: Box<dyn ErasedHandler>) {
		self.0.borrow_mut().push(handler);
	}

	pub(crate) fn clear(&self) {
		self.0.borrow_mut().clear();
	}

	pub(crate) fn borrow(&self) -> Ref<'_, Vec<Box<dyn ErasedHandler>>> {
		self.0.borrow()
	}

	pub(crate) fn unmet_expectations(&self) -> Vec<String> {
		self.0
			.borrow()
			.iter()
			.filter_map(|handler| handler.unmet_expectation())
			.collect()
	}
}

#[cfg(native)]
#[derive(Clone)]
pub(crate) struct SharedHandlers(Arc<Mutex<Vec<Box<dyn ErasedHandler>>>>);

#[cfg(native)]
impl SharedHandlers {
	pub(crate) fn new() -> Self {
		Self(Arc::new(Mutex::new(Vec::new())))
	}

	pub(crate) fn push(&self, handler: Box<dyn ErasedHandler>) {
		self.0
			.lock()
			.expect("MSW handler lock poisoned")
			.push(handler);
	}

	pub(crate) fn clear(&self) {
		self.0.lock().expect("MSW handler lock poisoned").clear();
	}

	pub(crate) fn lock(&self) -> MutexGuard<'_, Vec<Box<dyn ErasedHandler>>> {
		self.0.lock().expect("MSW handler lock poisoned")
	}

	/// Like [`Self::lock`], but usable from `Drop` after a panic poisoned the lock.
	pub(crate) fn unmet_expectations(&self) -> Vec<String> {
		let handlers = match self.0.lock() {
			Ok(handlers) => handlers,
			Err(poisoned) => poisoned.into_inner(),
		};
		handlers
			.iter()
			.filter_map(|handler| handler.unmet_expectation())
			.collect()
	}
}

#[cfg(wasm)]
#[derive(Clone)]
pub(crate) struct RecorderHandle(Rc<RefCell<RequestRecorder>>);

#[cfg(wasm)]
impl RecorderHandle {
	pub(crate) fn new() -> Self {
		Self(Rc::new(RefCell::new(RequestRecorder::new())))
	}

	pub(crate) fn record(&self, request: RecordedRequest) {
		self.0.borrow_mut().record(request);
	}

	pub(crate) fn clear(&self) {
		self.0.borrow_mut().clear();
	}

	pub(crate) fn all(&self) -> Vec<RecordedRequest> {
		self.0.borrow().all().to_vec()
	}
}

#[cfg(native)]
#[derive(Clone)]
pub(crate) struct RecorderHandle(Arc<Mutex<RequestRecorder>>);

#[cfg(native)]
impl RecorderHandle {
	pub(crate) fn new() -> Self {
		Self(Arc::new(Mutex::new(RequestRecorder::new())))
	}

	pub(crate) fn record(&self, request: RecordedRequest) {
		self.0
			.lock()
			.expect("MSW recorder lock poisoned")
			.record(request);
	}

	pub(crate) fn clear(&self) {
		self.0.lock().expect("MSW recorder lock poisoned").clear();
	}

	pub(crate) fn all(&self) -> Vec<RecordedRequest> {
		self.0
			.lock()
			.expect("MSW recorder lock poisoned")
			.all()
			.to_vec()
	}
}
