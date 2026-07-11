//! Fine-grained Reactive System for Reinhardt Framework
//!
//! This crate provides the core reactive primitives for building reactive applications.
//! The system is inspired by Leptos and Solid.js, offering fine-grained reactivity
//! without Virtual DOM overhead.
//!
//! ## Core Primitives
//!
//! - `Signal<T>` - A reactive value that can change over time
//! - `Effect` - A side effect that automatically reruns when dependencies change
//! - `Memo<T>` - A cached computation that updates when dependencies change
//!
//! ## Architecture
//!
//! The reactivity system is built on a pull-based model:
//!
//! 1. **Dependency Tracking**: When a Signal is read (`.get()`), it automatically registers
//!    the current observer (Effect or Memo) as a dependent.
//! 2. **Change Notification**: When a Signal changes (`.set()` or `.update()`), all dependent
//!    observers are notified and scheduled for re-execution.
//! 3. **Batching**: Multiple Signal changes can be batched into a single update cycle.
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_core::reactive::{Signal, Effect, Memo, ReactiveScope};
//!
//! ReactiveScope::run(|| {
//!     // Low-level node creation requires an active scope.
//!     let count = Signal::new(0);
//!     let name = Signal::new("Alice".to_string());
//!     let doubled = Memo::new(move || count.get() * 2);
//!
//!     Effect::new(move || {
//!         println!("{}: count = {}, doubled = {}",
//!             name.get(), count.get(), doubled.get());
//!     });
//!
//!     count.set(5);
//! });
//! ```
//!
//! ## no_std Compatibility
//!
//! This crate is `no_std` compatible by default, using `alloc` for heap allocations.
//! Standard library features can be enabled with the `std` feature flag.

extern crate alloc;

pub mod context;
pub mod deps;
pub mod effect;
pub mod memo;
pub mod runtime;
pub mod scope;
pub mod signal;

// Re-export main types
pub use context::{
	Context, ContextGuard, create_context, get_context, provide_context, remove_context,
};
pub use deps::{Deps, IntoDeps, Trackable};
pub use effect::Effect;
pub use memo::Memo;
pub use runtime::{EffectTiming, NodeId, NodeType, Observer, Runtime, batch, with_runtime};
pub use scope::{NodeKey, ReactiveScope, ReactiveScopeError, ScopeId, on_scope_dispose};
pub use signal::Signal;
