//! Dependency Injection support for Middleware

use async_trait::async_trait;
use reinhardt_apps::{Handler, Middleware, Request, Response, Result};
use reinhardt_di::{DiResult, Injectable, InjectionContext};
use std::sync::Arc;

/// Middleware with DI support
pub struct DiMiddleware<M: Middleware + Injectable> {
	middleware: Arc<M>,
}

impl<M: Middleware + Injectable> DiMiddleware<M> {
	/// Create a new DiMiddleware by resolving dependencies from the injection context
	///
	/// # Arguments
	///
	/// * `ctx` - The dependency injection context
	///
	/// # Examples
	///
	/// ```
	/// use std::sync::Arc;
	/// use reinhardt_middleware::di_support::{DiMiddleware, LoggingMiddleware};
	/// use reinhardt_di::{InjectionContext, SingletonScope};
	///
	/// # tokio_test::block_on(async {
	/// let singleton = Arc::new(SingletonScope::new());
	/// let ctx = InjectionContext::new(singleton);
	///
	/// let middleware = DiMiddleware::<LoggingMiddleware>::new(&ctx).await.unwrap();
	/// assert_eq!(middleware.inner().logger().prefix, "[APP]");
	/// # });
	/// ```
	pub async fn new(ctx: &InjectionContext) -> DiResult<Self> {
		let middleware = M::inject(ctx).await?;
		Ok(Self {
			middleware: Arc::new(middleware),
		})
	}
	/// Get a reference to the inner middleware
	///
	/// # Examples
	///
	/// ```
	/// use std::sync::Arc;
	/// use reinhardt_middleware::di_support::{DiMiddleware, LoggingMiddleware};
	/// use reinhardt_di::{InjectionContext, SingletonScope};
	///
	/// # tokio_test::block_on(async {
	/// let singleton = Arc::new(SingletonScope::new());
	/// let ctx = InjectionContext::new(singleton);
	///
	/// let middleware = DiMiddleware::<LoggingMiddleware>::new(&ctx).await.unwrap();
	/// let inner = middleware.inner();
	/// assert_eq!(inner.logger().prefix, "[APP]");
	/// # });
	/// ```
	pub fn inner(&self) -> &M {
		&self.middleware
	}
}

#[async_trait]
impl<M: Middleware + Injectable> Middleware for DiMiddleware<M> {
	async fn process(&self, request: Request, handler: Arc<dyn Handler>) -> Result<Response> {
		self.middleware.process(request, handler).await
	}
}

/// Middleware factory with DI
#[async_trait]
pub trait MiddlewareFactory: Send + Sync {
	type Middleware: Middleware;

	/// Create a new middleware instance with injected dependencies
	async fn create(&self, ctx: &InjectionContext) -> DiResult<Self::Middleware>;
}

/// Example: Logging middleware with injected logger
#[derive(Clone)]
pub struct Logger {
	pub prefix: String,
}

#[async_trait]
impl Injectable for Logger {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Logger {
			prefix: "[APP]".to_string(),
		})
	}
}

#[derive(Clone)]
pub struct LoggingMiddleware {
	logger: Logger,
}

impl LoggingMiddleware {
	pub fn logger(&self) -> &Logger {
		&self.logger
	}
}

#[async_trait]
impl Injectable for LoggingMiddleware {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let logger = Logger::inject(ctx).await?;
		Ok(LoggingMiddleware { logger })
	}
}

#[async_trait]
impl Middleware for LoggingMiddleware {
	async fn process(&self, request: Request, handler: Arc<dyn Handler>) -> Result<Response> {
		println!(
			"{} {} {}",
			self.logger.prefix,
			request.method,
			request.path()
		);
		handler.handle(request).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Uri, Version};
	use reinhardt_di::SingletonScope;

	struct TestHandler;

	#[async_trait]
	impl Handler for TestHandler {
		async fn handle(&self, _request: Request) -> Result<Response> {
			Ok(Response::ok())
		}
	}

	#[tokio::test]
	async fn test_logger_injection() {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		let logger = Logger::inject(&ctx).await.unwrap();
		assert_eq!(logger.prefix, "[APP]");
	}

	#[tokio::test]
	async fn test_logging_middleware_injection() {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		let middleware = LoggingMiddleware::inject(&ctx).await.unwrap();
		assert_eq!(middleware.logger.prefix, "[APP]");
	}

	#[tokio::test]
	async fn test_di_middleware() {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		let middleware = DiMiddleware::<LoggingMiddleware>::new(&ctx).await.unwrap();

		let request = Request::new(
			Method::GET,
			Uri::from_static("/test"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let handler = Arc::new(TestHandler);
		let response = middleware.process(request, handler).await.unwrap();

		assert_eq!(response.status, hyper::StatusCode::OK);
	}
}
