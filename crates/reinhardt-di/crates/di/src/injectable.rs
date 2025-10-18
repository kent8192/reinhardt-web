//! Injectable trait for dependencies

use crate::{context::InjectionContext, DiResult};

/// Injectable trait for dependencies.
///
/// This trait defines how a type can be injected as a dependency.
/// Types implementing this trait can be used with `Depends<T>`.
///
/// # Automatic Implementation
///
/// Types that implement `Default + Clone + Send + Sync + 'static` automatically
/// get an `Injectable` implementation that:
/// 1. Checks if the value is already cached in the request scope
/// 2. Checks if the value is available in the singleton scope
/// 3. Creates a new instance using `Default::default()`
///
/// This automatic behavior is similar to FastAPI's dependency injection,
/// where simple types can be auto-injected without explicit implementation.
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt_di::{Injectable, InjectionContext, DiResult, Depends};
///
/// // Automatic injection for types with Default + Clone
/// #[derive(Default, Clone)]
/// struct Config {
///     api_key: String,
/// }
///
/// // Config now has Injectable automatically
/// // Can be used directly: Depends<Config>
///
/// // Custom injection logic
/// struct Database {
///     pool: DbPool,
/// }
///
/// #[async_trait::async_trait]
/// impl Injectable for Database {
///     async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
///         // Custom logic here
///         Ok(Database {
///             pool: create_pool().await?,
///         })
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait Injectable: Sized + Send + Sync + 'static {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self>;
}

/// Automatic Injectable implementation for types with Default + Clone.
///
/// This blanket implementation allows any type that is `Default + Clone + Send + Sync + 'static`
/// to be automatically injectable without requiring manual implementation.
#[async_trait::async_trait]
impl<T> Injectable for T
where
    T: Default + Clone + Send + Sync + 'static,
{
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        use std::sync::Arc;

        // Try to get from request scope first (cached)
        if let Some(cached) = ctx.get_request::<Self>() {
            return Ok(Arc::try_unwrap(cached).unwrap_or_else(|arc| (*arc).clone()));
        }

        // Try to get from singleton scope
        if let Some(singleton) = ctx.get_singleton::<Self>() {
            return Ok(Arc::try_unwrap(singleton).unwrap_or_else(|arc| (*arc).clone()));
        }

        // Create new instance using Default
        let instance = Self::default();

        // Cache in request scope
        ctx.set_request(instance.clone());

        Ok(instance)
    }
}
