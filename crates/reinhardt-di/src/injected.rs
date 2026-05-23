//! Injection metadata types.
//!
//! `InjectionMetadata` and [`DependencyScope`] are shared between
//! [`Depends`](crate::Depends) (the canonical dependency resolver) and
//! historical consumers. The `Injected<T>` / `OptionalInjected<T>`
//! wrappers that previously lived in this module were removed in 0.2.0
//! per Issue #4520; use [`Depends<T>`](crate::Depends) and
//! `Option<Depends<T>>` instead.

/// Injection metadata
///
/// Tracks the scope and caching status of an injected dependency.
#[derive(Debug, Clone, Copy)]
pub struct InjectionMetadata {
	/// Dependency scope (Request or Singleton)
	pub scope: DependencyScope,
	/// Whether caching was enabled during resolution
	pub cached: bool,
}

/// Dependency scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyScope {
	/// Request-scoped dependency (lifetime tied to request)
	Request,
	/// Singleton-scoped dependency (shared across requests)
	Singleton,
}
