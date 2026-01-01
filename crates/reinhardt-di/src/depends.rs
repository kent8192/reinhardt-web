//! Depends wrapper for dependency injection
//!
//! FastAPI-inspired dependency injection wrapper that provides:
//! - Automatic dependency resolution
//! - Caching control via `use_cache` parameter
//! - Type-safe dependency injection
//!
//! ## Examples
//!
//! ```rust,no_run
//! use reinhardt_di::{Depends, DiResult, Injectable, InjectionContext, SingletonScope};
//! use async_trait::async_trait;
//! use std::sync::Arc;
//!
//! #[derive(Clone, Default)]
//! struct Config {
//!     database_url: String,
//! }
//!
//! #[async_trait]
//! impl Injectable for Config {
//!     async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
//!         Ok(Self::default())
//!     }
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let singleton = Arc::new(SingletonScope::new());
//! let ctx = InjectionContext::builder(singleton).build();
//!
//! // Basic usage - with caching (default)
//! let config = Depends::<Config>::builder().resolve(&ctx).await?;
//!
//! // Without caching - creates new instance every time
//! let config = Depends::<Config>::builder_no_cache().resolve(&ctx).await?;
//! # Ok(())
//! # }
//! ```

use crate::{DiResult, Injectable, context::InjectionContext};
use std::ops::Deref;
use std::sync::Arc;

/// Dependency injection wrapper similar to FastAPI's Depends.
///
/// Provides automatic dependency resolution with optional caching.
#[derive(Debug)]
pub struct Depends<T: Injectable> {
	inner: Arc<T>,
	use_cache: bool,
}

impl<T: Injectable> Depends<T>
where
	T: Clone,
{
	/// Create a new DependsBuilder with caching enabled (default behavior).
	///
	/// Similar to FastAPI's `Depends(dependency, use_cache=True)`.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_di::{Depends, injectable};
	///
	/// #[derive(Clone, Default)]
	/// #[injectable]
	/// struct Config {
	///     #[no_inject]
	///     value: String,
	/// }
	///
	/// let builder = Depends::<Config>::builder();
	/// ```
	pub fn builder() -> DependsBuilder<T> {
		DependsBuilder {
			use_cache: true,
			_phantom: std::marker::PhantomData,
		}
	}
	/// Create a new DependsBuilder with caching disabled.
	///
	/// Similar to FastAPI's `Depends(dependency, use_cache=False)`.
	/// Each call will create a new instance.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_di::{Depends, injectable};
	///
	/// #[derive(Clone, Default)]
	/// #[injectable]
	/// struct RequestData {
	///     #[no_inject]
	///     id: u32,
	/// }
	///
	/// let builder = Depends::<RequestData>::builder_no_cache();
	/// ```
	pub fn builder_no_cache() -> DependsBuilder<T> {
		DependsBuilder {
			use_cache: false,
			_phantom: std::marker::PhantomData,
		}
	}
	/// Resolve the dependency from the injection context.
	///
	/// This method will:
	/// 1. Check cache if `use_cache` is true
	/// 2. Call `T::inject(ctx)` if not cached or cache is disabled
	/// 3. Store in cache if `use_cache` is true
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_di::{Depends, InjectionContext, SingletonScope, injectable};
	///
	/// #[derive(Clone, Default)]
	/// #[injectable]
	/// struct Config {
	///     #[no_inject]
	///     value: String,
	/// }
	///
	/// # async fn example() {
	/// let singleton_scope = SingletonScope::new();
	/// let ctx = InjectionContext::builder(singleton_scope).build();
	/// let result = Depends::<Config>::resolve(&ctx, true).await;
	/// assert!(result.is_ok());
	/// # }
	/// ```
	pub async fn resolve(ctx: &InjectionContext, use_cache: bool) -> DiResult<Self> {
		let value = if use_cache {
			// Try to get from cache first
			if let Some(cached) = ctx.get_request::<T>() {
				Arc::try_unwrap(cached).unwrap_or_else(|arc| (*arc).clone())
			} else {
				let v = T::inject(ctx).await?;
				ctx.set_request(v.clone());
				v
			}
		} else {
			// Skip cache - always create new instance using inject_uncached()
			T::inject_uncached(ctx).await?
		};

		Ok(Self {
			inner: Arc::new(value),
			use_cache,
		})
	}
	/// Create a Depends from an existing value (for testing).
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_di::{Depends, injectable};
	///
	/// #[derive(Clone, Default)]
	/// #[injectable]
	/// struct Config {
	///     #[no_inject]
	///     value: String,
	/// }
	///
	/// let config = Config { value: "test".to_string() };
	/// let depends = Depends::from_value(config);
	/// assert_eq!(depends.value, "test");
	/// ```
	pub fn from_value(value: T) -> Self {
		Self {
			inner: Arc::new(value),
			use_cache: true,
		}
	}

	/// Extract the inner value from the Depends wrapper.
	///
	/// This method tries to unwrap the Arc. If the Arc has multiple strong references,
	/// it clones the inner value instead.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_di::{Depends, injectable};
	///
	/// #[derive(Clone, Default)]
	/// #[injectable]
	/// struct Config {
	///     #[no_inject]
	///     value: String,
	/// }
	///
	/// let config = Config { value: "test".to_string() };
	/// let depends = Depends::from_value(config);
	/// let inner = depends.into_inner();
	/// assert_eq!(inner.value, "test");
	/// ```
	pub fn into_inner(self) -> T {
		Arc::try_unwrap(self.inner).unwrap_or_else(|arc| (*arc).clone())
	}
}

/// Builder for Depends to support FastAPI-style API.
pub struct DependsBuilder<T: Injectable> {
	use_cache: bool,
	_phantom: std::marker::PhantomData<T>,
}

impl<T: Injectable> DependsBuilder<T>
where
	T: Clone,
{
	/// Resolve the dependency.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_di::{Depends, InjectionContext, SingletonScope, injectable};
	///
	/// #[derive(Clone, Default)]
	/// #[injectable]
	/// struct Config {
	///     #[no_inject]
	///     value: String,
	/// }
	///
	/// # async fn example() {
	/// let singleton_scope = SingletonScope::new();
	/// let ctx = InjectionContext::builder(singleton_scope).build();
	/// let builder = Depends::<Config>::builder();
	/// let result = builder.resolve(&ctx).await;
	/// assert!(result.is_ok());
	/// # }
	/// ```
	pub async fn resolve(self, ctx: &InjectionContext) -> DiResult<Depends<T>> {
		Depends::resolve(ctx, self.use_cache).await
	}
}

impl<T: Injectable> Deref for Depends<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl<T: Injectable> Clone for Depends<T> {
	fn clone(&self) -> Self {
		Self {
			inner: Arc::clone(&self.inner),
			use_cache: self.use_cache,
		}
	}
}
