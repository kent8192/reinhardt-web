//! Injection context for dependency resolution

use crate::scope::{RequestScope, SingletonScope};
use std::any::Any;
use std::sync::Arc;

// Re-export ParamContext and Request types for convenience
#[cfg(feature = "params")]
pub use reinhardt_params::{ParamContext, Request};

pub struct InjectionContext {
	request_scope: RequestScope,
	singleton_scope: Arc<SingletonScope>,
	/// HTTP request for parameter extraction
	#[cfg(feature = "params")]
	request: Option<Arc<Request>>,
	/// Parameter context for path/header/cookie extraction
	#[cfg(feature = "params")]
	param_context: Option<Arc<ParamContext>>,
}

/// Builder for constructing `InjectionContext` instances.
///
/// Provides a fluent API for building injection contexts with optional HTTP request support.
///
/// # Examples
///
/// ```
/// use reinhardt_di::{InjectionContext, SingletonScope};
///
/// let singleton_scope = SingletonScope::new();
/// let ctx = InjectionContext::builder(singleton_scope).build();
/// ```
pub struct InjectionContextBuilder {
	singleton_scope: Arc<SingletonScope>,
	#[cfg(feature = "params")]
	request: Option<Request>,
	#[cfg(feature = "params")]
	param_context: Option<ParamContext>,
}

impl InjectionContextBuilder {
	/// Set the HTTP request for this context.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_di::{InjectionContext, SingletonScope, Request};
	///
	/// let singleton_scope = SingletonScope::new();
	/// let request = Request::builder()
	///     .method(hyper::Method::GET)
	///     .uri("/")
	///     .build()
	///     .unwrap();
	///
	/// let ctx = InjectionContext::builder(singleton_scope)
	///     .with_request(request)
	///     .build();
	/// ```
	#[cfg(feature = "params")]
	pub fn with_request(mut self, request: Request) -> Self {
		self.request = Some(request);
		self
	}

	/// Set the parameter context for this context.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_di::{InjectionContext, SingletonScope, ParamContext};
	///
	/// let singleton_scope = SingletonScope::new();
	/// let param_context = ParamContext::new();
	///
	/// let ctx = InjectionContext::builder(singleton_scope)
	///     .with_param_context(param_context)
	///     .build();
	/// ```
	#[cfg(feature = "params")]
	pub fn with_param_context(mut self, param_context: ParamContext) -> Self {
		self.param_context = Some(param_context);
		self
	}

	/// Register a singleton instance in the context.
	///
	/// This allows explicit registration of pre-configured instances
	/// that will be shared across all requests.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_di::{InjectionContext, SingletonScope};
	/// use std::sync::Arc;
	///
	/// #[derive(Debug, Clone)]
	/// struct DatabaseConfig {
	///     url: String,
	/// }
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// let config = DatabaseConfig { url: "postgres://localhost".to_string() };
	///
	/// let ctx = InjectionContext::builder(singleton_scope)
	///     .singleton(config)
	///     .build();
	/// ```
	pub fn singleton<T: std::any::Any + Send + Sync>(self, instance: T) -> Self {
		self.singleton_scope.set(instance);
		self
	}

	/// Build the final `InjectionContext` instance.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_di::{InjectionContext, SingletonScope};
	///
	/// let singleton_scope = SingletonScope::new();
	/// let ctx = InjectionContext::builder(singleton_scope).build();
	/// ```
	pub fn build(self) -> InjectionContext {
		InjectionContext {
			request_scope: RequestScope::new(),
			singleton_scope: self.singleton_scope,
			#[cfg(feature = "params")]
			request: self.request.map(Arc::new),
			#[cfg(feature = "params")]
			param_context: self.param_context.map(Arc::new),
		}
	}
}

impl InjectionContext {
	/// Create a new `InjectionContextBuilder`.
	///
	/// This is the recommended way to construct an `InjectionContext`.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_di::{InjectionContext, SingletonScope};
	///
	/// let singleton_scope = SingletonScope::new();
	/// let ctx = InjectionContext::builder(singleton_scope).build();
	/// ```
	pub fn builder(singleton_scope: impl Into<Arc<SingletonScope>>) -> InjectionContextBuilder {
		InjectionContextBuilder {
			singleton_scope: singleton_scope.into(),
			#[cfg(feature = "params")]
			request: None,
			#[cfg(feature = "params")]
			param_context: None,
		}
	}

	/// Gets the HTTP request from the context.
	///
	/// Returns `None` if no request was set (e.g., when testing without HTTP context).
	/// Returns a reference to the request.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_di::{InjectionContext, SingletonScope, Request, ParamContext};
	///
	/// let singleton_scope = SingletonScope::new();
	/// let request = Request::builder()
	///     .method(hyper::Method::GET)
	///     .uri("/")
	///     .build()
	///     .unwrap();
	/// let param_context = ParamContext::new();
	///
	/// let ctx = InjectionContext::builder(singleton_scope)
	///     .with_request(request)
	///     .with_param_context(param_context)
	///     .build();
	///
	/// assert!(ctx.get_http_request().is_some());
	/// ```
	#[cfg(feature = "params")]
	pub fn get_http_request(&self) -> Option<&Request> {
		self.request.as_ref().map(|arc| arc.as_ref())
	}

	/// Gets the parameter context from the context.
	///
	/// Returns `None` if no parameter context was set.
	/// Returns a reference to the parameter context.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_di::{InjectionContext, SingletonScope, Request, ParamContext};
	///
	/// let singleton_scope = SingletonScope::new();
	/// let request = Request::builder()
	///     .method(hyper::Method::GET)
	///     .uri("/")
	///     .build()
	///     .unwrap();
	/// let param_context = ParamContext::new();
	///
	/// let ctx = InjectionContext::builder(singleton_scope)
	///     .with_request(request)
	///     .with_param_context(param_context)
	///     .build();
	///
	/// assert!(ctx.get_param_context().is_some());
	/// ```
	#[cfg(feature = "params")]
	pub fn get_param_context(&self) -> Option<&ParamContext> {
		self.param_context.as_ref().map(|arc| arc.as_ref())
	}

	/// Sets the HTTP request and parameter context.
	///
	/// This can be used to add HTTP context to an existing InjectionContext.
	/// The request and parameter context will be wrapped in Arc internally.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_di::{InjectionContext, SingletonScope, Request, ParamContext};
	///
	/// let singleton_scope = SingletonScope::new();
	/// let mut ctx = InjectionContext::builder(singleton_scope).build();
	///
	/// let request = Request::builder()
	///     .method(hyper::Method::GET)
	///     .uri("/")
	///     .build()
	///     .unwrap();
	/// let param_context = ParamContext::new();
	///
	/// ctx.set_http_request(request, param_context);
	/// ```
	#[cfg(feature = "params")]
	pub fn set_http_request(&mut self, request: Request, param_context: ParamContext) {
		self.request = Some(Arc::new(request));
		self.param_context = Some(Arc::new(param_context));
	}
	/// Retrieves a request-scoped value from the context.
	///
	/// Request-scoped values are cached only for the duration of a single request.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_di::{InjectionContext, SingletonScope};
	/// use std::sync::Arc;
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// let ctx = InjectionContext::builder(singleton_scope).build();
	///
	/// ctx.set_request(42i32);
	/// let value = ctx.get_request::<i32>().unwrap();
	/// assert_eq!(*value, 42);
	/// ```
	pub fn get_request<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
		self.request_scope.get::<T>()
	}
	/// Stores a value in the request scope.
	///
	/// The value is cached for the duration of the current request only.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_di::{InjectionContext, SingletonScope};
	/// use std::sync::Arc;
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// let ctx = InjectionContext::builder(singleton_scope).build();
	///
	/// ctx.set_request("request-data".to_string());
	/// assert!(ctx.get_request::<String>().is_some());
	/// ```
	pub fn set_request<T: Any + Send + Sync>(&self, value: T) {
		self.request_scope.set(value);
	}
	/// Retrieves a singleton value from the context.
	///
	/// Singleton values persist across all requests and are shared application-wide.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_di::{InjectionContext, SingletonScope};
	/// use std::sync::Arc;
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// singleton_scope.set(100u64);
	///
	/// let ctx = InjectionContext::builder(singleton_scope).build();
	/// let value = ctx.get_singleton::<u64>().unwrap();
	/// assert_eq!(*value, 100);
	/// ```
	pub fn get_singleton<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
		self.singleton_scope.get::<T>()
	}
	/// Stores a value in the singleton scope.
	///
	/// The value persists across all requests and is shared application-wide.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_di::{InjectionContext, SingletonScope};
	/// use std::sync::Arc;
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// let ctx = InjectionContext::builder(singleton_scope).build();
	///
	/// ctx.set_singleton("global-config".to_string());
	/// assert!(ctx.get_singleton::<String>().is_some());
	/// ```
	pub fn set_singleton<T: Any + Send + Sync>(&self, value: T) {
		self.singleton_scope.set(value);
	}
}

pub struct RequestContext {
	injection_ctx: InjectionContext,
}

impl RequestContext {
	/// Creates a new RequestContext with a shared singleton scope.
	///
	/// This is typically used to create a context for each incoming request.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_di::{RequestContext, SingletonScope};
	///
	/// let singleton_scope = SingletonScope::new();
	/// let request_ctx = RequestContext::new(singleton_scope);
	/// ```
	pub fn new(singleton_scope: SingletonScope) -> Self {
		Self {
			injection_ctx: InjectionContext::builder(singleton_scope).build(),
		}
	}
	/// Returns a reference to the underlying injection context.
	///
	/// This allows access to the dependency injection context for resolving dependencies.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_di::{RequestContext, SingletonScope};
	///
	/// let singleton_scope = SingletonScope::new();
	/// let request_ctx = RequestContext::new(singleton_scope);
	///
	/// let ctx = request_ctx.injection_context();
	/// ctx.set_request(42i32);
	/// ```
	pub fn injection_context(&self) -> &InjectionContext {
		&self.injection_ctx
	}
}
