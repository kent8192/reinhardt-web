//! Injection context for dependency resolution

use crate::scope::{RequestScope, SingletonScope};
use std::any::Any;
use std::sync::Arc;

// Re-export Request and ParamContext types for convenience
pub use reinhardt_apps::Request;
pub use reinhardt_params::ParamContext;

pub struct InjectionContext {
	request_scope: RequestScope,
	singleton_scope: Arc<SingletonScope>,
	/// HTTP request for parameter extraction
	request: Option<Arc<Request>>,
	/// Parameter context for path/header/cookie extraction
	param_context: Option<Arc<ParamContext>>,
}

impl InjectionContext {
	/// Creates a new InjectionContext with a shared singleton scope.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_di::{InjectionContext, SingletonScope};
	/// use std::sync::Arc;
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// let ctx = InjectionContext::new(singleton_scope);
	/// ```
	pub fn new(singleton_scope: Arc<SingletonScope>) -> Self {
		Self {
			request_scope: RequestScope::new(),
			singleton_scope,
			request: None,
			param_context: None,
		}
	}

	/// Creates a new InjectionContext with HTTP request and parameter context.
	///
	/// This is the preferred constructor when using FastAPI-style dependency injection
	/// with HTTP request parameter extraction.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_di::{InjectionContext, SingletonScope, Request, ParamContext};
	/// use std::sync::Arc;
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// let request = Arc::new(request); // Your HTTP request
	/// let param_context = Arc::new(ParamContext::new());
	///
	/// let ctx = InjectionContext::with_request(singleton_scope, request, param_context);
	/// ```
	pub fn with_request(
		singleton_scope: Arc<SingletonScope>,
		request: Arc<Request>,
		param_context: Arc<ParamContext>,
	) -> Self {
		Self {
			request_scope: RequestScope::new(),
			singleton_scope,
			request: Some(request),
			param_context: Some(param_context),
		}
	}

	/// Gets the HTTP request from the context.
	///
	/// Returns `None` if no request was set (e.g., when testing without HTTP context).
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_di::{InjectionContext, SingletonScope, Request, ParamContext};
	/// use std::sync::Arc;
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// let request = Arc::new(request);
	/// let param_context = Arc::new(ParamContext::new());
	///
	/// let ctx = InjectionContext::with_request(singleton_scope, request.clone(), param_context);
	/// assert!(ctx.get_http_request().is_some());
	/// ```
	pub fn get_http_request(&self) -> Option<&Arc<Request>> {
		self.request.as_ref()
	}

	/// Gets the parameter context from the context.
	///
	/// Returns `None` if no parameter context was set.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_di::{InjectionContext, SingletonScope, Request, ParamContext};
	/// use std::sync::Arc;
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// let request = Arc::new(request);
	/// let param_context = Arc::new(ParamContext::new());
	///
	/// let ctx = InjectionContext::with_request(singleton_scope, request, param_context.clone());
	/// assert!(ctx.get_param_context().is_some());
	/// ```
	pub fn get_param_context(&self) -> Option<&Arc<ParamContext>> {
		self.param_context.as_ref()
	}

	/// Sets the HTTP request and parameter context.
	///
	/// This can be used to add HTTP context to an existing InjectionContext.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_di::{InjectionContext, SingletonScope, Request, ParamContext};
	/// use std::sync::Arc;
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// let ctx = InjectionContext::new(singleton_scope);
	///
	/// let request = Arc::new(request);
	/// let param_context = Arc::new(ParamContext::new());
	/// ctx.set_http_request(request, param_context);
	/// ```
	pub fn set_http_request(&mut self, request: Arc<Request>, param_context: Arc<ParamContext>) {
		self.request = Some(request);
		self.param_context = Some(param_context);
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
	/// let ctx = InjectionContext::new(singleton_scope);
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
	/// let ctx = InjectionContext::new(singleton_scope);
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
	/// let ctx = InjectionContext::new(singleton_scope);
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
	/// let ctx = InjectionContext::new(singleton_scope);
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
	/// use std::sync::Arc;
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// let request_ctx = RequestContext::new(singleton_scope);
	/// ```
	pub fn new(singleton_scope: Arc<SingletonScope>) -> Self {
		Self {
			injection_ctx: InjectionContext::new(singleton_scope),
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
	/// use std::sync::Arc;
	///
	/// let singleton_scope = Arc::new(SingletonScope::new());
	/// let request_ctx = RequestContext::new(singleton_scope);
	///
	/// let ctx = request_ctx.injection_context();
	/// ctx.set_request(42i32);
	/// ```
	pub fn injection_context(&self) -> &InjectionContext {
		&self.injection_ctx
	}
}
