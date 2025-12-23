//! Reactive System
//!
//! This module provides a fine-grained reactivity system inspired by Leptos and Solid.js.
//! The core primitives are:
//!
//! - [`Signal<T>`](signal::Signal): A reactive value that can change over time
//! - [`Effect`](effect::Effect): A side effect that automatically reruns when dependencies change
//! - [`Memo<T>`](memo::Memo): A cached computation that automatically updates when dependencies change
//!
//! ## Architecture
//!
//! The reactivity system is built on a pull-based model:
//!
//! 1. **Dependency Tracking**: When a Signal is read (`.get()`), it automatically registers
//!    the current observer (Effect or Memo) as a dependent.
//! 2. **Change Notification**: When a Signal changes (`.set()` or `.update()`), all dependent
//!    observers are notified and scheduled for re-execution.
//! 3. **Batching**: Multiple Signal changes are batched into a single update cycle using
//!    micro-tasks for efficiency.
//!
//! ## Example
//!
//! ```ignore
//! use reinhardt_pages::reactive::{Signal, Effect, Memo};
//!
//! // Create signals
//! let count = Signal::new(0);
//! let name = Signal::new("Alice".to_string());
//!
//! // Create a memo (derived value)
//! let doubled = Memo::new(move || count.get() * 2);
//!
//! // Create an effect (side effect)
//! Effect::new(move || {
//!     println!("{}: count = {}, doubled = {}",
//!         name.get(), count.get(), doubled.get());
//! });
//! // Prints: "Alice: count = 0, doubled = 0"
//!
//! // Update signals - effect automatically reruns
//! count.set(5);
//! // Prints: "Alice: count = 5, doubled = 10"
//!
//! name.set("Bob".to_string());
//! // Prints: "Bob: count = 5, doubled = 10"
//! ```
//!
//! ## Fine-grained Reactivity
//!
//! Unlike Virtual DOM frameworks that re-render entire component trees, this system
//! uses **fine-grained reactivity** - only the specific DOM nodes that depend on
//! changed Signals are updated. This provides excellent performance for complex UIs.
//!
//! ## Memory Management
//!
//! All reactive nodes (Signals, Effects, Memos) automatically clean up their dependencies
//! when dropped, preventing memory leaks. However, Effects that capture references to
//! Signals will keep those Signals alive - be mindful of circular dependencies.

// Public modules using Rust 2024 Edition module system (no mod.rs files)
pub mod context;
pub mod effect;
pub mod hooks;
pub mod memo;
pub mod resource;
pub mod runtime;
pub mod signal;

// Re-export commonly used types
pub use context::{
	Context, ContextGuard, create_context, get_context, provide_context, remove_context,
};
pub use effect::Effect;
pub use memo::Memo;
pub use resource::{Resource, ResourceState};
#[cfg(target_arch = "wasm32")]
pub use resource::{create_resource, create_resource_with_deps};
pub use runtime::{NodeId, NodeType, with_runtime};
pub use signal::Signal;

// Re-export hooks
pub use hooks::{
	ActionState, Dispatch, OptimisticState, Ref, SetState, SharedSetState, SharedSignal,
	TransitionState, use_action_state, use_callback, use_context, use_debug_value,
	use_deferred_value, use_effect, use_effect_event, use_id, use_layout_effect, use_memo,
	use_optimistic, use_reducer, use_ref, use_shared_state, use_state, use_sync_external_store,
	use_transition,
};
