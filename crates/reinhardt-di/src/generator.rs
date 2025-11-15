//! Generator-based dependency resolution
//!
//! This module provides generator syntax for lazy dependency resolution,
//! allowing dependencies to be resolved on-demand in a streaming fashion.
//!
//! # Note
//!
//! **Workaround for Unstable Native Async Yield:**
//!
//! Rust's native async generators (with `yield` keyword) are not yet stable as of 2025.
//! This implementation uses the `genawaiter` crate as a workaround to provide
//! generator-like functionality on stable Rust.
//!
//! When Rust's native async generators become stable, this implementation should be
//! migrated to use the native syntax for better performance and ergonomics.
//!
//! Tracking issue: https://github.com/rust-lang/rust/issues/79024
//!
//! # Examples
//!
//! ```rust,ignore
//! use reinhardt_di::generator::DependencyGenerator;
//!
//! // Create a generator that yields dependencies one by one
//! let gen = DependencyGenerator::new(|co| async move {
//!     let db = resolve_database().await;
//!     co.yield_(db).await;
//!
//!     let cache = resolve_cache().await;
//!     co.yield_(cache).await;
//!
//!     let service = resolve_service().await;
//!     co.yield_(service).await;
//! });
//!
//! // Consume dependencies as they become available
//! while let Some(dep) = gen.next().await {
//!     // Use dependency
//! }
//! ```

#[cfg(feature = "generator")]
use genawaiter::GeneratorState;
#[cfg(feature = "generator")]
use genawaiter::sync::{Co, Gen};
#[cfg(feature = "generator")]
use std::future::Future;
#[cfg(feature = "generator")]
use std::marker::PhantomData;
#[cfg(feature = "generator")]
use std::pin::Pin;

/// Generator-based dependency resolver
///
/// Provides lazy, streaming dependency resolution using generators.
///
/// # Note
///
/// This uses `genawaiter` as a workaround for unstable native async yield.
#[cfg(feature = "generator")]
pub struct DependencyGenerator<T, R> {
	generator: Gen<T, (), Pin<Box<dyn Future<Output = R> + Send + 'static>>>,
	_phantom: PhantomData<(T, R)>,
}

#[cfg(feature = "generator")]
impl<T, R> DependencyGenerator<T, R>
where
	T: 'static,
	R: 'static,
{
	/// Create a new dependency generator
	///
	/// # Arguments
	///
	/// * `producer` - Async function that yields dependencies using the `Co` (coroutine) handle
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// let gen = DependencyGenerator::new(|co| async move {
	///     let db = Database::connect().await;
	///     co.yield_(db).await;
	/// });
	/// ```
	pub fn new<F>(producer: F) -> Self
	where
		F: FnOnce(Co<T>) -> Pin<Box<dyn Future<Output = R> + Send + 'static>> + Send + 'static,
	{
		Self {
			generator: Gen::new(producer),
			_phantom: PhantomData,
		}
	}

	/// Get the next dependency from the generator
	///
	/// Returns `None` when the generator is exhausted.
	pub async fn next(&mut self) -> Option<T> {
		match self.generator.async_resume().await {
			GeneratorState::Yielded(value) => Some(value),
			GeneratorState::Complete(_) => None,
		}
	}

	/// Collect all remaining dependencies into a vector
	pub async fn collect(mut self) -> Vec<T> {
		let mut deps = Vec::new();
		while let Some(dep) = self.next().await {
			deps.push(dep);
		}
		deps
	}
}

/// Dependency stream for request-scoped resolution
///
/// Provides async streaming of dependencies with lazy evaluation.
///
/// # Note
///
/// This uses `genawaiter` as a workaround for unstable native async yield.
#[cfg(feature = "generator")]
pub struct DependencyStream<T> {
	generator: Gen<T, (), Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,
	_phantom: PhantomData<T>,
}

#[cfg(feature = "generator")]
impl<T> DependencyStream<T>
where
	T: 'static,
{
	/// Create a new dependency stream
	pub fn new<F>(producer: F) -> Self
	where
		F: FnOnce(Co<T>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + 'static,
	{
		Self {
			generator: Gen::new(producer),
			_phantom: PhantomData,
		}
	}

	/// Stream the next dependency
	pub async fn next(&mut self) -> Option<T> {
		match self.generator.async_resume().await {
			GeneratorState::Yielded(value) => Some(value),
			GeneratorState::Complete(_) => None,
		}
	}

	/// Check if stream has more dependencies
	pub async fn is_empty(&mut self) -> bool {
		// Peek without consuming
		matches!(
			self.generator.async_resume().await,
			GeneratorState::Complete(_)
		)
	}
}

/// Request-scoped dependency resolver with generator
///
/// Resolves dependencies lazily for a specific request context.
///
/// # Note
///
/// This uses `genawaiter` as a workaround for unstable native async yield.
#[cfg(feature = "generator")]
pub struct RequestScopedGenerator<T> {
	request_id: String,
	stream: DependencyStream<T>,
}

#[cfg(feature = "generator")]
impl<T> RequestScopedGenerator<T>
where
	T: 'static,
{
	/// Create a new request-scoped generator
	pub fn new(request_id: String, stream: DependencyStream<T>) -> Self {
		Self { request_id, stream }
	}

	/// Get request ID
	pub fn request_id(&self) -> &str {
		&self.request_id
	}

	/// Resolve next dependency for this request
	pub async fn resolve_next(&mut self) -> Option<T> {
		self.stream.next().await
	}
}

#[cfg(feature = "generator")]
#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_dependency_generator_basic() {
		let mut generator = DependencyGenerator::new(|co| {
			Box::pin(async move {
				co.yield_(1).await;
				co.yield_(2).await;
				co.yield_(3).await;
			})
		});

		assert_eq!(generator.next().await, Some(1));
		assert_eq!(generator.next().await, Some(2));
		assert_eq!(generator.next().await, Some(3));
		assert_eq!(generator.next().await, None);
	}

	#[tokio::test]
	async fn test_dependency_generator_collect() {
		let generator = DependencyGenerator::new(|co| {
			Box::pin(async move {
				co.yield_(1).await;
				co.yield_(2).await;
				co.yield_(3).await;
			})
		});

		let deps = generator.collect().await;
		assert_eq!(deps, vec![1, 2, 3]);
	}

	#[tokio::test]
	async fn test_dependency_stream() {
		let mut stream = DependencyStream::new(|co| {
			Box::pin(async move {
				co.yield_("db".to_string()).await;
				co.yield_("cache".to_string()).await;
			})
		});

		assert_eq!(stream.next().await, Some("db".to_string()));
		assert_eq!(stream.next().await, Some("cache".to_string()));
		assert_eq!(stream.next().await, None);
	}

	#[tokio::test]
	async fn test_request_scoped_generator() {
		let stream = DependencyStream::new(|co| {
			Box::pin(async move {
				co.yield_("dependency1".to_string()).await;
				co.yield_("dependency2".to_string()).await;
			})
		});

		let mut generator = RequestScopedGenerator::new("request-123".to_string(), stream);

		assert_eq!(generator.request_id(), "request-123");
		assert_eq!(
			generator.resolve_next().await,
			Some("dependency1".to_string())
		);
		assert_eq!(
			generator.resolve_next().await,
			Some("dependency2".to_string())
		);
		assert_eq!(generator.resolve_next().await, None);
	}

	#[tokio::test]
	async fn test_async_operations_in_generator() {
		let generator = DependencyGenerator::new(|co| {
			Box::pin(async move {
				// Simulate async database connection
				co.yield_("database".to_string()).await;

				// Simulate async cache connection
				co.yield_("cache".to_string()).await;
			})
		});

		let deps = generator.collect().await;
		assert_eq!(deps, vec!["database".to_string(), "cache".to_string()]);
	}
}
