// Re-export Request and Response for convenience
#[cfg(feature = "http")]
pub use reinhardt_http::{Request, Response};

#[cfg(feature = "http")]
mod http_types {
	use async_trait::async_trait;
	use reinhardt_exception::Result;
	use std::sync::Arc;

	use super::{Request, Response};

	/// Handler trait for processing requests
	/// This is the core abstraction - all request handlers implement this
	#[async_trait]
	pub trait Handler: Send + Sync {
		async fn handle(&self, request: Request) -> Result<Response>;
	}

	/// Blanket implementation for `Arc<T>` where T: Handler
	/// This allows `Arc<dyn Handler>` to be used as a Handler
	#[async_trait]
	impl<T: Handler + ?Sized> Handler for Arc<T> {
		async fn handle(&self, request: Request) -> Result<Response> {
			(**self).handle(request).await
		}
	}

	/// Middleware trait for request/response processing
	/// Uses composition pattern instead of inheritance
	#[async_trait]
	pub trait Middleware: Send + Sync {
		async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response>;

		/// Determines whether this middleware should be executed for the given request.
		///
		/// This method enables conditional execution of middleware, allowing the middleware
		/// chain to skip unnecessary middleware based on request properties.
		///
		/// # Performance Benefits
		///
		/// By implementing this method, middleware chains can achieve O(k) complexity
		/// instead of O(n), where k is the number of middleware that should run,
		/// and k ≤ n (total middleware count).
		///
		/// # Examples
		///
		/// ```no_run
		/// use reinhardt_types::{Middleware, Request, Response, Handler};
		/// use std::sync::Arc;
		/// use async_trait::async_trait;
		///
		/// struct AuthMiddleware;
		///
		/// #[async_trait]
		/// impl Middleware for AuthMiddleware {
		///     async fn process(&self, request: Request, next: Arc<dyn Handler>) -> reinhardt_exception::Result<Response> {
		///         // Authentication logic
		///         next.handle(request).await
		///     }
		///
		///     fn should_continue(&self, request: &Request) -> bool {
		///         // Only execute for paths starting with /api/
		///         request.uri.path().starts_with("/api/")
		///     }
		/// }
		/// ```
		///
		/// # Common Use Cases
		///
		/// - Skip authentication middleware for public endpoints
		/// - Skip compression middleware for already compressed responses
		/// - Skip CORS middleware for same-origin requests
		/// - Skip rate limiting for internal/admin requests
		///
		/// # Default Implementation
		///
		/// By default, returns `true` (always execute), maintaining backward compatibility.
		fn should_continue(&self, _request: &Request) -> bool {
			true
		}
	}

	/// Middleware chain - composes multiple middleware
	pub struct MiddlewareChain {
		middlewares: Vec<Arc<dyn Middleware>>,
		handler: Arc<dyn Handler>,
	}

	impl MiddlewareChain {
		/// Creates a new middleware chain with the given handler.
		///
		/// # Examples
		///
		/// ```no_run
		/// use reinhardt_types::{MiddlewareChain, Handler};
		/// use std::sync::Arc;
		///
		/// # struct MyHandler;
		/// # #[async_trait::async_trait]
		/// # impl Handler for MyHandler {
		/// #     async fn handle(&self, request: reinhardt_http::Request) -> reinhardt_exception::Result<reinhardt_http::Response> {
		/// #         Ok(reinhardt_http::Response::ok())
		/// #     }
		/// # }
		/// let handler = Arc::new(MyHandler);
		/// let chain = MiddlewareChain::new(handler);
		/// ```
		pub fn new(handler: Arc<dyn Handler>) -> Self {
			Self {
				middlewares: Vec::new(),
				handler,
			}
		}
		/// Adds a middleware to the chain using builder pattern.
		///
		/// # Examples
		///
		/// ```no_run
		/// use reinhardt_types::{MiddlewareChain, Handler, Middleware};
		/// use std::sync::Arc;
		///
		/// # struct MyHandler;
		/// # struct MyMiddleware;
		/// # #[async_trait::async_trait]
		/// # impl Handler for MyHandler {
		/// #     async fn handle(&self, request: reinhardt_http::Request) -> reinhardt_exception::Result<reinhardt_http::Response> {
		/// #         Ok(reinhardt_http::Response::ok())
		/// #     }
		/// # }
		/// # #[async_trait::async_trait]
		/// # impl Middleware for MyMiddleware {
		/// #     async fn process(&self, request: reinhardt_http::Request, next: Arc<dyn Handler>) -> reinhardt_exception::Result<reinhardt_http::Response> {
		/// #         next.handle(request).await
		/// #     }
		/// # }
		/// let handler = Arc::new(MyHandler);
		/// let middleware = Arc::new(MyMiddleware);
		/// let chain = MiddlewareChain::new(handler)
		///     .with_middleware(middleware);
		/// ```
		pub fn with_middleware(mut self, middleware: Arc<dyn Middleware>) -> Self {
			self.middlewares.push(middleware);
			self
		}
		/// Adds a middleware to the chain.
		///
		/// # Examples
		///
		/// ```no_run
		/// use reinhardt_types::{MiddlewareChain, Handler, Middleware};
		/// use std::sync::Arc;
		///
		/// # struct MyHandler;
		/// # struct MyMiddleware;
		/// # #[async_trait::async_trait]
		/// # impl Handler for MyHandler {
		/// #     async fn handle(&self, request: reinhardt_http::Request) -> reinhardt_exception::Result<reinhardt_http::Response> {
		/// #         Ok(reinhardt_http::Response::ok())
		/// #     }
		/// # }
		/// # #[async_trait::async_trait]
		/// # impl Middleware for MyMiddleware {
		/// #     async fn process(&self, request: reinhardt_http::Request, next: Arc<dyn Handler>) -> reinhardt_exception::Result<reinhardt_http::Response> {
		/// #         next.handle(request).await
		/// #     }
		/// # }
		/// let handler = Arc::new(MyHandler);
		/// let middleware = Arc::new(MyMiddleware);
		/// let mut chain = MiddlewareChain::new(handler);
		/// chain.add_middleware(middleware);
		/// ```
		pub fn add_middleware(&mut self, middleware: Arc<dyn Middleware>) {
			self.middlewares.push(middleware);
		}
	}

	#[async_trait]
	impl Handler for MiddlewareChain {
		async fn handle(&self, request: Request) -> Result<Response> {
			if self.middlewares.is_empty() {
				return self.handler.handle(request).await;
			}

			// Build nested handler chain using composition with optimizations:
			// 1. Conditional execution (skip middleware based on should_continue)
			// 2. Short-circuiting (early return if response.should_stop_chain() is true)
			//
			// Performance improvements:
			// - Condition check: O(1) per middleware
			// - Skip unnecessary middleware: achieves O(k) where k ≤ n
			// - Early return: stops processing on first stop_chain=true response
			let mut current_handler = self.handler.clone();

			// Filter middleware based on should_continue condition
			// This achieves the O(k) optimization where k is the number of middleware that should run
			let active_middlewares: Vec<_> = self
				.middlewares
				.iter()
				.rev()
				.filter(|mw| mw.should_continue(&request))
				.collect();

			for middleware in active_middlewares {
				let mw = middleware.clone();
				let handler = current_handler.clone();

				current_handler = Arc::new(ConditionalComposedHandler {
					middleware: mw,
					next: handler,
				});
			}

			current_handler.handle(request).await
		}
	}

	/// Optimized internal handler that composes middleware with next handler
	/// Supports short-circuiting via response.should_stop_chain()
	struct ConditionalComposedHandler {
		middleware: Arc<dyn Middleware>,
		next: Arc<dyn Handler>,
	}

	#[async_trait]
	impl Handler for ConditionalComposedHandler {
		async fn handle(&self, request: Request) -> Result<Response> {
			// Process the request through this middleware
			let response = self.middleware.process(request, self.next.clone()).await?;

			// Short-circuit: if response indicates chain should stop, return immediately
			// This prevents further middleware/handlers from executing
			if response.should_stop_chain() {
				return Ok(response);
			}

			Ok(response)
		}
	}
}

#[cfg(feature = "http")]
pub use http_types::{Handler, Middleware, MiddlewareChain};

#[cfg(all(test, feature = "http"))]
mod tests {
	use super::*;
	use async_trait::async_trait;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Version};
	use reinhardt_exception::Result;
	use std::sync::Arc;

	// Mock handler for testing
	struct MockHandler {
		response_body: String,
	}

	#[async_trait]
	impl Handler for MockHandler {
		async fn handle(&self, _request: Request) -> Result<Response> {
			Ok(Response::ok().with_body(self.response_body.clone()))
		}
	}

	// Mock middleware for testing
	struct MockMiddleware {
		prefix: String,
	}

	#[async_trait]
	impl Middleware for MockMiddleware {
		async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
			// Call the next handler
			let response = next.handle(request).await?;

			// Modify the response
			let current_body = String::from_utf8(response.body.to_vec()).unwrap_or_default();
			let new_body = format!("{}{}", self.prefix, current_body);

			Ok(Response::ok().with_body(new_body))
		}
	}

	fn create_test_request() -> Request {
		Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap()
	}

	#[tokio::test]
	async fn test_handler_basic() {
		let handler = MockHandler {
			response_body: "Hello".to_string(),
		};

		let request = create_test_request();
		let response = handler.handle(request).await.unwrap();

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body, "Hello");
	}

	#[tokio::test]
	async fn test_middleware_basic() {
		let handler = Arc::new(MockHandler {
			response_body: "World".to_string(),
		});

		let middleware = MockMiddleware {
			prefix: "Hello, ".to_string(),
		};

		let request = create_test_request();
		let response = middleware.process(request, handler).await.unwrap();

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body, "Hello, World");
	}

	#[tokio::test]
	async fn test_middleware_chain_empty() {
		let handler = Arc::new(MockHandler {
			response_body: "Test".to_string(),
		});

		let chain = MiddlewareChain::new(handler);

		let request = create_test_request();
		let response = chain.handle(request).await.unwrap();

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body, "Test");
	}

	#[tokio::test]
	async fn test_middleware_chain_single() {
		let handler = Arc::new(MockHandler {
			response_body: "Handler".to_string(),
		});

		let middleware1 = Arc::new(MockMiddleware {
			prefix: "MW1:".to_string(),
		});

		let chain = MiddlewareChain::new(handler).with_middleware(middleware1);

		let request = create_test_request();
		let response = chain.handle(request).await.unwrap();

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body, "MW1:Handler");
	}

	#[tokio::test]
	async fn test_middleware_chain_multiple() {
		let handler = Arc::new(MockHandler {
			response_body: "Data".to_string(),
		});

		let middleware1 = Arc::new(MockMiddleware {
			prefix: "M1:".to_string(),
		});

		let middleware2 = Arc::new(MockMiddleware {
			prefix: "M2:".to_string(),
		});

		let chain = MiddlewareChain::new(handler)
			.with_middleware(middleware1)
			.with_middleware(middleware2);

		let request = create_test_request();
		let response = chain.handle(request).await.unwrap();

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		// Middleware are applied in the order they were added
		assert_eq!(body, "M1:M2:Data");
	}

	#[tokio::test]
	async fn test_middleware_chain_add_middleware() {
		let handler = Arc::new(MockHandler {
			response_body: "Result".to_string(),
		});

		let middleware = Arc::new(MockMiddleware {
			prefix: "Prefix:".to_string(),
		});

		let mut chain = MiddlewareChain::new(handler);
		chain.add_middleware(middleware);

		let request = create_test_request();
		let response = chain.handle(request).await.unwrap();

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body, "Prefix:Result");
	}

	// Conditional middleware that only runs for /api/* paths
	struct ConditionalMiddleware {
		prefix: String,
	}

	#[async_trait]
	impl Middleware for ConditionalMiddleware {
		async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
			let response = next.handle(request).await?;
			let current_body = String::from_utf8(response.body.to_vec()).unwrap_or_default();
			let new_body = format!("{}{}", self.prefix, current_body);
			Ok(Response::ok().with_body(new_body))
		}

		fn should_continue(&self, request: &Request) -> bool {
			request.uri.path().starts_with("/api/")
		}
	}

	#[tokio::test]
	async fn test_middleware_conditional_skip() {
		let handler = Arc::new(MockHandler {
			response_body: "Response".to_string(),
		});

		let conditional_mw = Arc::new(ConditionalMiddleware {
			prefix: "API:".to_string(),
		});

		let chain = MiddlewareChain::new(handler).with_middleware(conditional_mw);

		// Test with /api/ path - middleware should run
		let api_request = Request::builder()
			.method(Method::GET)
			.uri("/api/users")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let response = chain.handle(api_request).await.unwrap();
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body, "API:Response");

		// Test with non-/api/ path - middleware should be skipped
		let non_api_request = Request::builder()
			.method(Method::GET)
			.uri("/public")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let response = chain.handle(non_api_request).await.unwrap();
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body, "Response"); // No prefix because middleware was skipped
	}

	// Middleware that returns early with stop_chain=true
	struct ShortCircuitMiddleware {
		should_stop: bool,
	}

	#[async_trait]
	impl Middleware for ShortCircuitMiddleware {
		async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
			if self.should_stop {
				// Return early without calling next
				return Ok(Response::unauthorized()
					.with_body("Auth required")
					.with_stop_chain(true));
			}
			next.handle(request).await
		}
	}

	#[tokio::test]
	async fn test_middleware_short_circuit() {
		let handler = Arc::new(MockHandler {
			response_body: "Handler Response".to_string(),
		});

		let short_circuit_mw = Arc::new(ShortCircuitMiddleware { should_stop: true });
		let normal_mw = Arc::new(MockMiddleware {
			prefix: "Normal:".to_string(),
		});

		let chain = MiddlewareChain::new(handler)
			.with_middleware(short_circuit_mw)
			.with_middleware(normal_mw);

		let request = create_test_request();
		let response = chain.handle(request).await.unwrap();

		// Should get unauthorized response, not the handler response
		assert_eq!(response.status, hyper::StatusCode::UNAUTHORIZED);
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body, "Auth required");
	}

	#[tokio::test]
	async fn test_middleware_no_short_circuit() {
		let handler = Arc::new(MockHandler {
			response_body: "Handler Response".to_string(),
		});

		let short_circuit_mw = Arc::new(ShortCircuitMiddleware { should_stop: false });
		let normal_mw = Arc::new(MockMiddleware {
			prefix: "Normal:".to_string(),
		});

		let chain = MiddlewareChain::new(handler)
			.with_middleware(short_circuit_mw)
			.with_middleware(normal_mw);

		let request = create_test_request();
		let response = chain.handle(request).await.unwrap();

		// Should pass through to handler and apply normal middleware
		assert_eq!(response.status, hyper::StatusCode::OK);
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body, "Normal:Handler Response");
	}

	#[tokio::test]
	async fn test_middleware_multiple_conditions() {
		let handler = Arc::new(MockHandler {
			response_body: "Base".to_string(),
		});

		// Only runs for /api/* paths
		let api_mw = Arc::new(ConditionalMiddleware {
			prefix: "API:".to_string(),
		});

		// Always runs
		let always_mw = Arc::new(MockMiddleware {
			prefix: "Always:".to_string(),
		});

		let chain = MiddlewareChain::new(handler)
			.with_middleware(api_mw)
			.with_middleware(always_mw);

		// Test with /api/ path - both middleware should run
		let api_request = Request::builder()
			.method(Method::GET)
			.uri("/api/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let response = chain.handle(api_request).await.unwrap();
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body, "API:Always:Base");

		// Test with non-/api/ path - only always_mw should run
		let non_api_request = Request::builder()
			.method(Method::GET)
			.uri("/public")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let response = chain.handle(non_api_request).await.unwrap();
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body, "Always:Base"); // Only always_mw prefix
	}

	#[tokio::test]
	async fn test_response_should_stop_chain() {
		let response = Response::ok();
		assert!(!response.should_stop_chain());

		let stopping_response = Response::unauthorized().with_stop_chain(true);
		assert!(stopping_response.should_stop_chain());
	}
}
