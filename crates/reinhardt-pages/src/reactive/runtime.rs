//! Reactive Runtime
//!
//! This module provides the core reactive runtime for managing Signal dependencies,
//! Effect execution, and update scheduling.
//!
//! ## Architecture
//!
//! The reactive system is based on a pull-based reactivity model similar to Leptos and Solid.js:
//!
//! 1. **Observer Stack**: Tracks currently executing Effects
//! 2. **Dependency Tracking**: Automatically records dependencies when Signal::get() is called
//! 3. **Update Scheduling**: Batches multiple Signal changes into a single update cycle
//! 4. **Micro-task Execution**: Uses browser micro-tasks for efficient batching
//!
//! ## Example
//!
//! ```ignore
//! use reinhardt_pages::reactive::{Signal, Effect, Runtime};
//!
//! // Create a signal
//! let count = Signal::new(0);
//!
//! // Create an effect that automatically tracks dependencies
//! Effect::new(move || {
//!     // This get() call automatically registers the dependency
//!     println!("Count is: {}", count.get());
//! });
//!
//! // Update the signal - the effect will automatically re-run
//! count.set(42);
//! ```

use core::cell::RefCell;
use core::sync::atomic::{AtomicUsize, Ordering};

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Unique identifier for reactive nodes (Signals, Effects, Memos)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(usize);

impl NodeId {
	/// Create a new unique NodeId
	pub fn new() -> Self {
		static COUNTER: AtomicUsize = AtomicUsize::new(0);
		Self(COUNTER.fetch_add(1, Ordering::Relaxed))
	}
}

impl Default for NodeId {
	fn default() -> Self {
		Self::new()
	}
}

/// Type of reactive node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
	/// A Signal node (source of reactivity)
	Signal,
	/// An Effect node (side effect that runs when dependencies change)
	Effect,
	/// A Memo node (cached computation)
	Memo,
}

/// Observer represents a currently executing Effect or Memo
pub struct Observer {
	/// Unique identifier for this observer
	pub id: NodeId,
	/// Type of this observer
	pub node_type: NodeType,
	/// Cleanup function to run when dependencies change (not used yet)
	pub cleanup: Option<()>,
}

impl Clone for Observer {
	fn clone(&self) -> Self {
		Self {
			id: self.id,
			node_type: self.node_type,
			cleanup: None, // Cleanup functions are not cloneable
		}
	}
}

/// Dependency graph node
#[derive(Debug, Default)]
pub(crate) struct DependencyNode {
	/// IDs of nodes that depend on this node
	pub(crate) subscribers: Vec<NodeId>,
	/// IDs of nodes this node depends on
	pub(crate) dependencies: Vec<NodeId>,
}

/// Global reactive runtime
///
/// This struct manages the reactive dependency graph and update scheduling.
/// It uses thread-local storage to maintain separate runtime state per thread.
pub struct Runtime {
	/// Observer stack for tracking currently executing effects
	observer_stack: RefCell<Vec<Observer>>,
	/// Dependency graph: NodeId -> DependencyNode
	pub(crate) dependency_graph: RefCell<BTreeMap<NodeId, DependencyNode>>,
	/// Pending updates (nodes that need to be re-executed)
	pub(crate) pending_updates: RefCell<Vec<NodeId>>,
	/// Whether an update is currently scheduled
	pub(crate) update_scheduled: RefCell<bool>,
}

impl Runtime {
	/// Create a new Runtime instance
	pub fn new() -> Self {
		Self {
			observer_stack: RefCell::new(Vec::new()),
			dependency_graph: RefCell::new(BTreeMap::new()),
			pending_updates: RefCell::new(Vec::new()),
			update_scheduled: RefCell::new(false),
		}
	}

	/// Get the current observer (the currently executing Effect or Memo)
	pub fn current_observer(&self) -> Option<NodeId> {
		self.observer_stack
			.borrow()
			.last()
			.map(|observer| observer.id)
	}

	/// Push an observer onto the stack
	///
	/// This should be called when starting to execute an Effect or Memo.
	pub fn push_observer(&self, observer: Observer) {
		self.observer_stack.borrow_mut().push(observer);
	}

	/// Pop an observer from the stack
	///
	/// This should be called when finishing execution of an Effect or Memo.
	pub fn pop_observer(&self) -> Option<Observer> {
		self.observer_stack.borrow_mut().pop()
	}

	/// Track a dependency between the current observer and a signal
	///
	/// This is called automatically when Signal::get() is invoked.
	///
	/// # Arguments
	///
	/// * `signal_id` - ID of the Signal being accessed
	pub fn track_dependency(&self, signal_id: NodeId) {
		if let Some(observer_id) = self.current_observer() {
			let mut graph = self.dependency_graph.borrow_mut();

			// Add signal -> observer edge (signal has a new subscriber)
			let signal_node = graph.entry(signal_id).or_default();
			if !signal_node.subscribers.contains(&observer_id) {
				signal_node.subscribers.push(observer_id);
			}

			// Add observer -> signal edge (observer depends on signal)
			let observer_node = graph.entry(observer_id).or_default();
			if !observer_node.dependencies.contains(&signal_id) {
				observer_node.dependencies.push(signal_id);
			}
		}
	}

	/// Notify that a Signal has changed
	///
	/// This schedules all subscribers (Effects/Memos that depend on this Signal) for re-execution.
	///
	/// # Arguments
	///
	/// * `signal_id` - ID of the Signal that changed
	pub fn notify_signal_change(&self, signal_id: NodeId) {
		let graph = self.dependency_graph.borrow();
		if let Some(node) = graph.get(&signal_id) {
			for &subscriber_id in &node.subscribers {
				self.schedule_update(subscriber_id);
			}
		}
	}

	/// Schedule a node for update
	///
	/// The actual update will be performed in a batched micro-task.
	///
	/// # Arguments
	///
	/// * `node_id` - ID of the node to update
	pub fn schedule_update(&self, node_id: NodeId) {
		let mut pending = self.pending_updates.borrow_mut();
		if !pending.contains(&node_id) {
			pending.push(node_id);
		}

		// Schedule flush if not already scheduled
		if !*self.update_scheduled.borrow() {
			*self.update_scheduled.borrow_mut() = true;

			#[cfg(target_arch = "wasm32")]
			{
				// In WASM environment, use spawn_local to schedule flush as microtask
				wasm_bindgen_futures::spawn_local(async {
					RUNTIME.with(|rt| rt.flush_updates_enhanced());
				});
			}

			#[cfg(not(target_arch = "wasm32"))]
			{
				// On non-WASM platforms, no automatic scheduling
				// Tests should call flush_updates_enhanced() manually
			}
		}
	}

	/// Flush all pending updates (basic version)
	///
	/// This is a basic implementation that clears the pending updates queue.
	/// For actual Effect execution, use `flush_updates_enhanced()` which is
	/// implemented in the effect module.
	///
	/// Note: This method is kept for backward compatibility and simple testing.
	/// Production code should use `flush_updates_enhanced()` instead.
	pub fn flush_updates(&self) {
		*self.update_scheduled.borrow_mut() = false;

		// Take all pending updates
		let pending = core::mem::take(&mut *self.pending_updates.borrow_mut());

		// Clear the queue (actual execution is handled by flush_updates_enhanced)
		drop(pending);
	}

	/// Clear dependencies for a node
	///
	/// This should be called before re-executing an Effect/Memo to clear old dependencies.
	///
	/// # Arguments
	///
	/// * `node_id` - ID of the node whose dependencies should be cleared
	pub fn clear_dependencies(&self, node_id: NodeId) {
		let mut graph = self.dependency_graph.borrow_mut();

		// Get the current dependencies
		if let Some(node) = graph.get(&node_id) {
			let dependencies = node.dependencies.clone();

			// Remove this node from all signal subscribers
			for &dep_id in &dependencies {
				if let Some(dep_node) = graph.get_mut(&dep_id) {
					dep_node.subscribers.retain(|&id| id != node_id);
				}
			}
		}

		// Clear the dependencies list
		if let Some(node) = graph.get_mut(&node_id) {
			node.dependencies.clear();
		}
	}

	/// Remove a node from the dependency graph
	///
	/// This should be called when a Signal/Effect/Memo is dropped.
	///
	/// # Arguments
	///
	/// * `node_id` - ID of the node to remove
	pub fn remove_node(&self, node_id: NodeId) {
		self.clear_dependencies(node_id);
		self.dependency_graph.borrow_mut().remove(&node_id);
	}

	/// Check if a node exists in the dependency graph (for testing)
	pub fn has_node(&self, node_id: NodeId) -> bool {
		self.dependency_graph.borrow().contains_key(&node_id)
	}

	/// Get the number of subscribers for a node (for testing)
	pub fn subscriber_count(&self, node_id: NodeId) -> usize {
		self.dependency_graph
			.borrow()
			.get(&node_id)
			.map(|node| node.subscribers.len())
			.unwrap_or(0)
	}
}

impl Default for Runtime {
	fn default() -> Self {
		Self::new()
	}
}

// Thread-local runtime instance
//
// In WASM, there is only one thread, so this effectively provides a global runtime.
// On non-WASM platforms, each thread gets its own runtime instance.
thread_local! {
	static RUNTIME: Runtime = Runtime::new();
}

/// Get a reference to the global runtime
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::runtime::with_runtime;
///
/// with_runtime(|rt| {
///     rt.track_dependency(signal_id);
/// });
/// ```
pub fn with_runtime<F, R>(f: F) -> R
where
	F: FnOnce(&Runtime) -> R,
{
	RUNTIME.with(f)
}

/// Try to access the global runtime (safe version for Drop implementations)
///
/// Returns None if the thread-local storage has been destroyed.
pub(crate) fn try_with_runtime<F, R>(f: F) -> Option<R>
where
	F: FnOnce(&Runtime) -> R,
{
	RUNTIME.try_with(f).ok()
}

#[cfg(test)]
mod tests {
	use super::*;
	use serial_test::serial;

	#[test]
	#[serial]
	fn test_node_id_uniqueness() {
		let id1 = NodeId::new();
		let id2 = NodeId::new();
		let id3 = NodeId::new();

		assert_ne!(id1, id2);
		assert_ne!(id2, id3);
		assert_ne!(id1, id3);
	}

	#[test]
	#[serial]
	fn test_runtime_observer_stack() {
		let runtime = Runtime::new();

		assert!(runtime.current_observer().is_none());

		let observer1 = Observer {
			id: NodeId::new(),
			node_type: NodeType::Effect,
			cleanup: None,
		};
		let id1 = observer1.id;

		runtime.push_observer(observer1);
		assert_eq!(runtime.current_observer(), Some(id1));

		let observer2 = Observer {
			id: NodeId::new(),
			node_type: NodeType::Effect,
			cleanup: None,
		};
		let id2 = observer2.id;

		runtime.push_observer(observer2);
		assert_eq!(runtime.current_observer(), Some(id2));

		runtime.pop_observer();
		assert_eq!(runtime.current_observer(), Some(id1));

		runtime.pop_observer();
		assert!(runtime.current_observer().is_none());
	}

	#[test]
	#[serial]
	fn test_dependency_tracking() {
		let runtime = Runtime::new();

		let signal_id = NodeId::new();
		let effect_id = NodeId::new();

		// Push effect observer
		runtime.push_observer(Observer {
			id: effect_id,
			node_type: NodeType::Effect,
			cleanup: None,
		});

		// Track dependency
		runtime.track_dependency(signal_id);

		// Verify dependency was recorded
		let graph = runtime.dependency_graph.borrow();
		let signal_node = graph.get(&signal_id).unwrap();
		assert!(signal_node.subscribers.contains(&effect_id));

		let effect_node = graph.get(&effect_id).unwrap();
		assert!(effect_node.dependencies.contains(&signal_id));
	}

	#[test]
	#[serial]
	fn test_notify_signal_change() {
		let runtime = Runtime::new();

		let signal_id = NodeId::new();
		let effect_id = NodeId::new();

		// Manually add dependency
		{
			let mut graph = runtime.dependency_graph.borrow_mut();
			graph
				.entry(signal_id)
				.or_default()
				.subscribers
				.push(effect_id);
		}

		// Notify change
		runtime.notify_signal_change(signal_id);

		// Verify update was scheduled
		let pending = runtime.pending_updates.borrow();
		assert!(pending.contains(&effect_id));
	}

	#[test]
	#[serial]
	fn test_clear_dependencies() {
		let runtime = Runtime::new();

		let signal_id = NodeId::new();
		let effect_id = NodeId::new();

		// Manually add dependency
		{
			let mut graph = runtime.dependency_graph.borrow_mut();
			graph
				.entry(signal_id)
				.or_default()
				.subscribers
				.push(effect_id);
			graph
				.entry(effect_id)
				.or_default()
				.dependencies
				.push(signal_id);
		}

		// Clear dependencies
		runtime.clear_dependencies(effect_id);

		// Verify dependencies were cleared
		let graph = runtime.dependency_graph.borrow();
		let signal_node = graph.get(&signal_id).unwrap();
		assert!(!signal_node.subscribers.contains(&effect_id));

		let effect_node = graph.get(&effect_id).unwrap();
		assert!(effect_node.dependencies.is_empty());
	}
}
