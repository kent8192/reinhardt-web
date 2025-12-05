//! Injectable trait for dependencies

use crate::{DiResult, context::InjectionContext};

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
// Automatic injection for types with Default + Clone
/// #[derive(Default, Clone)]
/// struct Config {
///     api_key: String,
/// }
///
// Config now has Injectable automatically
// Can be used directly: Depends<Config>
///
// Custom injection logic
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

	/// Inject without using cache (for `cache = false` support).
	///
	/// This method creates a new instance without checking or updating any cache.
	/// By default, it delegates to `inject()`, but types can override this
	/// to provide cache-free injection.
	async fn inject_uncached(ctx: &InjectionContext) -> DiResult<Self> {
		Self::inject(ctx).await
	}
}
