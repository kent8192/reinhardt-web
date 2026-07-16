//! Pages-owned navigation preparation and commit coordination.

use crate::cancellation::{AbortableTaskGuard, CancellationSource};
use crate::reactive::Signal;
use crate::reactive::hooks::router::NavigateError;
use crate::router::NavigationType;
use crate::router::loader::{LoaderStore, RouteLoaderError, route_context};
use crate::router::loader_registry::{LoaderConsumer, LoaderRegistry, execute_loader};
use futures_util::future::join_all;
use reinhardt_urls::routers::client_router::{ClientRouteTreeMatch, ClientRouter};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// Pop and initial intents are supplied by the browser launcher; native unit
// tests exercise the synchronous push path only.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NavigationIntent {
	Initial,
	Push,
	Replace,
	Pop { target_index: i64 },
}

impl NavigationIntent {
	fn navigation_type(self) -> NavigationType {
		match self {
			Self::Initial => NavigationType::Initial,
			Self::Push => NavigationType::Push,
			Self::Replace => NavigationType::Replace,
			Self::Pop { .. } => NavigationType::Pop,
		}
	}

	fn entry_index(self) -> i64 {
		match self {
			Self::Pop { target_index } => target_index,
			Self::Initial | Self::Push | Self::Replace => 0,
		}
	}
}

// Fields mirror the navigation attempt contract and are retained until the
// current generation commits or is cancelled. Some fields are diagnostic
// ownership anchors rather than read by the commit path itself.
#[allow(dead_code)]
struct NavigationAttempt {
	generation: u64,
	intent: NavigationIntent,
	path: String,
	matched: ClientRouteTreeMatch,
	cancellation: CancellationSource,
	_task: AbortableTaskGuard,
}

/// Coordinates asynchronous route-loader preparation with synchronous URL
/// matching and commit operations.
pub(crate) struct NavigationCoordinator {
	router: Rc<ClientRouter>,
	registry: LoaderRegistry,
	next_generation: Cell<u64>,
	committed_index: Cell<i64>,
	pending: Signal<bool>,
	error: Signal<Option<RouteLoaderError>>,
	active_attempt: RefCell<Option<NavigationAttempt>>,
	mounted_store: RefCell<Option<LoaderStore>>,
	restoring_pop: Cell<bool>,
	// Prefetch work is retained for the coordinator lifetime and consumed by
	// the link-interceptor path when it is installed in a browser.
	#[allow(dead_code)]
	prefetch_tasks: RefCell<Vec<(CancellationSource, AbortableTaskGuard)>>,
}

// Accessors are used by the launcher, transition hooks, and prefetch path on
// WASM; native builds keep them available for deterministic integration tests.
#[allow(dead_code)]
impl NavigationCoordinator {
	pub(crate) fn new(router: Rc<ClientRouter>) -> Result<Rc<Self>, RouteLoaderError> {
		let registry = LoaderRegistry::global()
			.map_err(|error| RouteLoaderError::with_status(error.to_string(), 500))?;
		Ok(Rc::new(Self {
			router,
			registry,
			next_generation: Cell::new(0),
			committed_index: Cell::new(0),
			pending: Signal::new(false),
			error: Signal::new(None),
			active_attempt: RefCell::new(None),
			mounted_store: RefCell::new(None),
			restoring_pop: Cell::new(false),
			prefetch_tasks: RefCell::new(Vec::new()),
		}))
	}

	pub(crate) fn pending(&self) -> Signal<bool> {
		self.pending
	}

	pub(crate) fn error(&self) -> Signal<Option<RouteLoaderError>> {
		self.error
	}

	pub(crate) fn mounted_store(&self) -> Option<LoaderStore> {
		self.mounted_store.borrow().clone()
	}

	/// Returns the currently committed history index used for legacy popstate
	/// entries that do not carry framework metadata.
	pub(crate) fn committed_index(&self) -> i64 {
		self.committed_index.get()
	}

	/// Consumes the one-shot pop generated while restoring a failed navigation.
	pub(crate) fn consume_restoration_pop(&self) -> bool {
		self.restoring_pop.replace(false)
	}

	/// Restores the initial route's prepared loader values from the SSR state.
	///
	/// Hydration is intentionally strict for matched loader routes: rendering a
	/// destination without its entry-blocking values would violate the loader
	/// contract and would cause the generated component binding to panic.
	#[cfg(wasm)]
	pub(crate) fn hydrate_initial_store(&self, path: &str) -> Result<(), RouteLoaderError> {
		let Some(matched) = self.router.match_tree(path) else {
			return Ok(());
		};
		if matched.loader_ids().is_empty() {
			return Ok(());
		}
		let context = crate::hydration::HydrationContext::from_window().map_err(|error| {
			RouteLoaderError::with_status(
				format!("route loader hydration state is unavailable: {error}"),
				500,
			)
		})?;
		let store = LoaderStore::new();
		for id in matched.loader_ids() {
			let value = context.get_route_loader_state(id.as_str()).ok_or_else(|| {
				RouteLoaderError::with_status(
					format!("route loader `{}` is missing from SSR state", id.as_str()),
					500,
				)
			})?;
			let prepared = self.registry.hydrate(*id, value)?;
			store.insert_prepared(prepared);
		}
		self.mounted_store.borrow_mut().replace(store);
		Ok(())
	}

	pub(crate) fn navigate(
		self: &Rc<Self>,
		path: String,
		intent: NavigationIntent,
	) -> Result<(), NavigateError> {
		let Some(matched) = self.router.match_tree(&path) else {
			return Err(NavigateError::RouterRejected(format!(
				"no route matches `{path}`"
			)));
		};

		self.cancel_active_attempt();
		let generation = self.next_generation.get().wrapping_add(1);
		self.next_generation.set(generation);
		self.error.set(None);

		if matched.loader_ids().is_empty() {
			self.pending.set(false);
			self.commit_success(generation, path, intent, matched, LoaderStore::new());
			return Ok(());
		}

		self.pending.set(true);
		let cancellation = CancellationSource::new();
		let cancellation_handle = cancellation.handle();
		let ids = matched.loader_ids().to_vec();
		let context = route_context(&matched);
		let coordinator = Rc::clone(self);
		let path_for_task = path.clone();
		let matched_for_task = matched.clone();
		let task_cancellation = cancellation_handle.clone();
		let task = crate::cancellation::spawn_abortable_task(async move {
			let futures = ids.into_iter().map(|id| {
				execute_loader(
					&coordinator.registry,
					id,
					&context,
					task_cancellation.clone(),
					LoaderConsumer::Navigation(generation),
				)
			});
			let results = join_all(futures).await;
			if task_cancellation.is_cancelled() {
				return;
			}
			let store = LoaderStore::new();
			for result in results {
				match result {
					Ok(prepared) => store.insert_prepared(prepared),
					Err(error) => {
						coordinator.finish_error(generation, error);
						return;
					}
				}
			}
			coordinator.commit_success(generation, path_for_task, intent, matched_for_task, store);
		});

		*self.active_attempt.borrow_mut() = Some(NavigationAttempt {
			generation,
			intent,
			path,
			matched,
			cancellation,
			_task: task,
		});
		Ok(())
	}

	pub(crate) fn prefetch(self: &Rc<Self>, path: String) -> Result<(), NavigateError> {
		let Some(matched) = self.router.match_tree(&path) else {
			return Ok(());
		};
		let ids = matched.loader_ids().to_vec();
		if ids.is_empty() {
			return Ok(());
		}
		let context = route_context(&matched);
		let cancellation = CancellationSource::new();
		let handle = cancellation.handle();
		let coordinator = Rc::clone(self);
		let task = crate::cancellation::spawn_abortable_task(async move {
			let futures = ids.into_iter().map(|id| {
				execute_loader(
					&coordinator.registry,
					id,
					&context,
					handle.clone(),
					LoaderConsumer::Prefetch,
				)
			});
			let _ = join_all(futures).await;
		});
		self.prefetch_tasks.borrow_mut().push((cancellation, task));
		Ok(())
	}

	fn cancel_active_attempt(&self) {
		if let Some(attempt) = self.active_attempt.borrow_mut().take() {
			attempt.cancellation.cancel();
			// Dropping the attempt's task guard aborts any obsolete future.
		}
	}

	fn is_current_generation(&self, generation: u64) -> bool {
		self.next_generation.get() == generation
	}

	fn finish_error(&self, generation: u64, error: RouteLoaderError) {
		if !self.is_current_generation(generation) {
			return;
		}
		self.pending.set(false);
		self.error.set(Some(error));
		if let Some(attempt) = self.active_attempt.borrow().as_ref()
			&& let NavigationIntent::Pop { target_index } = attempt.intent
		{
			let delta = self.committed_index.get().saturating_sub(target_index);
			if delta != 0 {
				self.restoring_pop.set(true);
				if reinhardt_urls::routers::client_router::history::go(
					delta.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
				)
				.is_err()
				{
					self.restoring_pop.set(false);
				}
			}
		}
		self.active_attempt.borrow_mut().take();
	}

	fn commit_success(
		&self,
		generation: u64,
		path: String,
		intent: NavigationIntent,
		matched: ClientRouteTreeMatch,
		store: LoaderStore,
	) {
		if !self.is_current_generation(generation) {
			return;
		}
		let result = crate::router::loader::with_loader_store(&store, || {
			self.router.commit_match(
				&path,
				&matched,
				intent.navigation_type(),
				intent.entry_index(),
			)
		});
		if let Err(error) = result {
			self.finish_error(
				generation,
				RouteLoaderError::with_status(error.to_string(), 500),
			);
			return;
		}
		self.mounted_store.borrow_mut().replace(store);
		self.committed_index.set(match intent {
			NavigationIntent::Push => self.committed_index.get().saturating_add(1),
			NavigationIntent::Replace | NavigationIntent::Initial => self.committed_index.get(),
			NavigationIntent::Pop { target_index } => target_index,
		});
		self.pending.set(false);
		self.error.set(None);
		self.active_attempt.borrow_mut().take();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_core::reactive::ReactiveScope;

	#[test]
	fn no_loader_navigation_commits_synchronously() {
		ReactiveScope::run(|| {
			let router = Rc::new(
				ClientRouter::new().route("home", "/", || reinhardt_core::page::Page::empty()),
			);
			let coordinator = NavigationCoordinator::new(router.clone()).expect("registry builds");
			coordinator
				.navigate("/".to_string(), NavigationIntent::Push)
				.expect("known route commits");
			assert_eq!(router.current_path().get(), "/");
			assert!(!coordinator.pending().get());
		});
	}
}
