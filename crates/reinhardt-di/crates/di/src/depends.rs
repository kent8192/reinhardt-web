//! Depends wrapper for dependency injection
//!
//! FastAPI-inspired dependency injection wrapper that provides:
//! - Automatic dependency resolution
//! - Caching control via `use_cache` parameter
//! - Type-safe dependency injection
//!
//! ## Examples
//!
//! ```rust,ignore
//! use reinhardt_di::{Depends, Injectable, InjectionContext};
//!
//! #[derive(Clone, Default)]
//! struct Config {
//!     database_url: String,
//! }
//!
//! // Basic usage - with caching (default)
//! let config = Depends::<Config>::new().resolve(ctx).await?;
//!
//! // Without caching - creates new instance every time
//! let config = Depends::<Config>::no_cache().resolve(ctx).await?;
//! ```

use crate::{context::InjectionContext, DiResult, Injectable};
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
    /// Create a new Depends with caching enabled (default behavior).
    ///
    /// Similar to FastAPI's `Depends(dependency, use_cache=True)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_di::Depends;
    ///
    /// #[derive(Clone, Default)]
    /// struct Config {
    ///     value: String,
    /// }
    ///
    /// let builder = Depends::<Config>::new();
    /// ```
    pub fn new() -> DependsBuilder<T> {
        DependsBuilder {
            use_cache: true,
            _phantom: std::marker::PhantomData,
        }
    }
    /// Create a new Depends with caching disabled.
    ///
    /// Similar to FastAPI's `Depends(dependency, use_cache=False)`.
    /// Each call will create a new instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_di::Depends;
    ///
    /// #[derive(Clone, Default)]
    /// struct RequestData {
    ///     id: u32,
    /// }
    ///
    /// let builder = Depends::<RequestData>::no_cache();
    /// ```
    pub fn no_cache() -> DependsBuilder<T> {
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
    /// ```
    /// use reinhardt_di::{Depends, InjectionContext};
    ///
    /// #[derive(Clone, Default)]
    /// struct Config {
    ///     value: String,
    /// }
    ///
    /// # tokio_test::block_on(async {
    /// let ctx = InjectionContext::new();
    /// let result = Depends::<Config>::resolve(&ctx, true).await;
    /// assert!(result.is_ok());
    /// # });
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
    /// ```
    /// use reinhardt_di::Depends;
    ///
    /// #[derive(Clone, Default)]
    /// struct Config {
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
    /// ```
    /// use reinhardt_di::{Depends, InjectionContext};
    ///
    /// #[derive(Clone, Default)]
    /// struct Config {
    ///     value: String,
    /// }
    ///
    /// # tokio_test::block_on(async {
    /// let ctx = InjectionContext::new();
    /// let builder = Depends::<Config>::new();
    /// let result = builder.resolve(&ctx).await;
    /// assert!(result.is_ok());
    /// # });
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
