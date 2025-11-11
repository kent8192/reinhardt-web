//! Dependency Injection support for ViewSets

use crate::viewset::ViewSet;
use async_trait::async_trait;
use reinhardt_core::apps::{Request, Response, Result};
use reinhardt_core::di::{Depends, DiError, DiResult, Injectable, InjectionContext};
use std::sync::Arc;

/// ViewSet with DI support
pub struct DiViewSet<V: ViewSet + Injectable + Clone> {
	viewset: Depends<V>,
}

impl<V: ViewSet + Injectable + Clone> DiViewSet<V> {
	/// Create a new DiViewSet by resolving dependencies
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_viewsets::{DiViewSet, ViewSet};
	/// use reinhardt_core::di::{Injectable, InjectionContext, SingletonScope, DiResult};
	/// use reinhardt_core::apps::{Request, Response, Result};
	/// use std::sync::Arc;
	/// use async_trait::async_trait;
	///
	/// #[derive(Clone)]
	/// struct MyViewSet {
	///     basename: String,
	/// }
	///
	/// impl MyViewSet {
	///     fn new(basename: &str) -> Self {
	///         Self { basename: basename.to_string() }
	///     }
	/// }
	///
	/// #[async_trait]
	/// impl ViewSet for MyViewSet {
	///     fn get_basename(&self) -> &str {
	///         &self.basename
	///     }
	///
	///     async fn dispatch(&self, _request: Request, _action: reinhardt_viewsets::Action) -> Result<Response> {
	///         Ok(Response::ok())
	///     }
	/// }
	///
	/// #[async_trait]
	/// impl Injectable for MyViewSet {
	///     async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
	///         Ok(MyViewSet::new("my_resource"))
	///     }
	/// }
	///
	/// # tokio_test::block_on(async {
	/// let singleton = Arc::new(SingletonScope::new());
	/// let ctx = InjectionContext::new(singleton);
	///
	/// let di_viewset = DiViewSet::<MyViewSet>::new(&ctx).await.unwrap();
	/// assert_eq!(di_viewset.get_basename(), "my_resource");
	/// # });
	/// ```
	pub async fn new(ctx: &InjectionContext) -> DiResult<Self> {
		let viewset = Depends::<V>::resolve(ctx, true).await?;
		Ok(Self { viewset })
	}
	/// Get the inner viewset
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_viewsets::{DiViewSet, ViewSet};
	/// use reinhardt_core::di::{Injectable, InjectionContext, SingletonScope, DiResult};
	/// use reinhardt_core::apps::{Request, Response, Result};
	/// use std::sync::Arc;
	/// use async_trait::async_trait;
	///
	/// #[derive(Clone)]
	/// struct MyViewSet {
	///     basename: String,
	/// }
	///
	/// impl MyViewSet {
	///     fn new(basename: &str) -> Self {
	///         Self { basename: basename.to_string() }
	///     }
	/// }
	///
	/// #[async_trait]
	/// impl ViewSet for MyViewSet {
	///     fn get_basename(&self) -> &str {
	///         &self.basename
	///     }
	///
	///     async fn dispatch(&self, _request: Request, _action: reinhardt_viewsets::Action) -> Result<Response> {
	///         Ok(Response::ok())
	///     }
	/// }
	///
	/// #[async_trait]
	/// impl Injectable for MyViewSet {
	///     async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
	///         Ok(MyViewSet::new("my_resource"))
	///     }
	/// }
	///
	/// # tokio_test::block_on(async {
	/// let singleton = Arc::new(SingletonScope::new());
	/// let ctx = InjectionContext::new(singleton);
	///
	/// let di_viewset = DiViewSet::<MyViewSet>::new(&ctx).await.unwrap();
	/// let inner_viewset = di_viewset.inner();
	/// assert_eq!(inner_viewset.get_basename(), "my_resource");
	/// # });
	/// ```
	pub fn inner(&self) -> &V {
		&self.viewset
	}
}

#[async_trait]
impl<V: ViewSet + Injectable + Clone> ViewSet for DiViewSet<V> {
	fn get_basename(&self) -> &str {
		self.viewset.get_basename()
	}

	async fn dispatch(&self, request: Request, action: crate::Action) -> Result<Response> {
		self.viewset.dispatch(request, action).await
	}
}

/// Trait for creating ViewSets with dependency injection
#[async_trait]
pub trait ViewSetFactory: Send + Sync {
	type ViewSet: ViewSet;

	/// Create a new viewset instance with injected dependencies
	async fn create(&self, ctx: &InjectionContext) -> DiResult<Self::ViewSet>;
}

/// Database connection injectable dependency
///
/// Provides database connection pooling using sqlx::SqlitePool.
/// This can be injected into ViewSets and other handlers.
///
/// # Examples
///
/// ```rust
/// use reinhardt_viewsets::di_support::DatabaseConnection;
/// use reinhardt_core::di::{Injectable, InjectionContext, SingletonScope};
/// use std::sync::Arc;
///
/// # tokio_test::block_on(async {
/// let singleton = Arc::new(SingletonScope::new());
/// let ctx = InjectionContext::new(singleton);
///
/// // DatabaseConnection will be automatically injected when requested
/// let db = DatabaseConnection::inject(&ctx).await.unwrap();
/// assert!(!db.pool.is_closed());
/// # });
/// ```
#[derive(Clone)]
pub struct DatabaseConnection {
	/// Database connection pool (SQLite)
	pub pool: Arc<sqlx::SqlitePool>,
}

impl DatabaseConnection {
	/// Create a new DatabaseConnection from a connection URL
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_viewsets::di_support::DatabaseConnection;
	///
	/// # tokio_test::block_on(async {
	/// let db = DatabaseConnection::new("sqlite::memory:").await.unwrap();
	/// assert!(!db.pool.is_closed());
	/// # });
	/// ```
	pub async fn new(url: &str) -> DiResult<Self> {
		let pool = sqlx::SqlitePool::connect(url)
			.await
			.map_err(|e| DiError::ProviderError(format!("Failed to connect to database: {}", e)))?;

		Ok(DatabaseConnection {
			pool: Arc::new(pool),
		})
	}

	/// Get a connection from the pool
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_viewsets::di_support::DatabaseConnection;
	///
	/// # tokio_test::block_on(async {
	/// let db = DatabaseConnection::new("sqlite::memory:").await.unwrap();
	/// let conn = db.get_connection().await.unwrap();
	/// # });
	/// ```
	pub async fn get_connection(&self) -> DiResult<sqlx::pool::PoolConnection<sqlx::Sqlite>> {
		self.pool.acquire().await.map_err(|e| {
			DiError::ProviderError(format!("Failed to acquire database connection: {}", e))
		})
	}
}

#[async_trait]
impl Injectable for DatabaseConnection {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Try to get database connection from singleton scope
		if let Some(db) = ctx.get_singleton::<DatabaseConnection>() {
			return Ok((*db).clone());
		}

		// Fallback: try to get database URL from environment or context
		let db_url =
			std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());

		let db = DatabaseConnection::new(&db_url).await?;

		// Cache in singleton scope for future requests
		ctx.set_singleton(db.clone());

		Ok(db)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::GenericViewSet;
	use reinhardt_core::apps::{Request, Response};
	use reinhardt_core::di::SingletonScope;

	#[derive(Clone)]
	struct TestHandler {
		db: DatabaseConnection,
	}

	#[async_trait]
	impl Injectable for TestHandler {
		async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
			let db = DatabaseConnection::inject(ctx).await?;
			Ok(TestHandler { db })
		}
	}

	impl TestHandler {
		#[allow(dead_code)]
		async fn handle(&self, _request: Request) -> Result<Response> {
			// Return success response indicating database connection is available
			Ok(Response::ok().with_json(&serde_json::json!({
				"status": "ok",
				"database": "connected"
			}))?)
		}
	}

	type TestViewSet = GenericViewSet<TestHandler>;

	#[async_trait]
	impl Injectable for TestViewSet {
		async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
			let handler = TestHandler::inject(ctx).await?;
			Ok(GenericViewSet::new("test", handler))
		}
	}

	#[tokio::test]
	async fn test_database_connection_injection() {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		let db = DatabaseConnection::inject(&ctx).await.unwrap();
		// Verify that we have a valid pool
		assert!(!db.pool.is_closed());
	}

	#[tokio::test]
	async fn test_handler_with_injected_db() {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		let handler = TestHandler::inject(&ctx).await.unwrap();
		// Verify that handler has a valid database connection
		assert!(!handler.db.pool.is_closed());
	}

	#[tokio::test]
	async fn test_database_connection_from_env() {
		// Set DATABASE_URL environment variable
		// SAFETY: This is safe for testing purposes in an isolated test environment
		unsafe {
			std::env::set_var("DATABASE_URL", "sqlite::memory:");
		}

		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		let db = DatabaseConnection::inject(&ctx).await.unwrap();
		assert!(!db.pool.is_closed());

		// Cleanup
		// SAFETY: This is safe for testing purposes in an isolated test environment
		unsafe {
			std::env::remove_var("DATABASE_URL");
		}
	}
}
