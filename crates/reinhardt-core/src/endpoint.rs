//! Endpoint metadata trait for HTTP Method Macros
//!
//! This module provides the `EndpointInfo` trait that HTTP Method Macros
//! (#[get], #[post], etc.) implement to provide route metadata.

use hyper::Method;

/// Trait for endpoint metadata used by HTTP Method Macros
///
/// This trait is automatically implemented by HTTP Method Macros (#[get], #[post], etc.)
/// for the generated View types. It provides the route path, HTTP method, and name
/// for URL reversal.
///
/// # Examples
///
/// The HTTP Method Macro generates a View type that implements this trait:
///
/// ```rust,no_run
/// # use reinhardt_macros::get;
/// # use reinhardt_di::params::Path;
/// # use reinhardt_views::ViewResult;
/// # use reinhardt_http::Response;
/// # use reinhardt_core::endpoint::EndpointInfo;
/// # use hyper::Method;
/// #[get("/users/{id}/", name = "get_user")]
/// pub async fn get_user(Path(id): Path<i64>) -> ViewResult<Response> {
///     Ok(Response::ok())
/// }
///
/// // Generates:
/// pub struct GetUserView;
///
/// impl EndpointInfo for GetUserView {
///     fn path() -> &'static str { "/users/{id}/" }
///     fn method() -> Method { Method::GET }
///     fn name() -> &'static str { "get_user" }
/// }
/// ```
pub trait EndpointInfo: Send + Sync {
	/// Returns the route path pattern
	///
	/// Example: "/users/{id}/"
	fn path() -> &'static str;

	/// Returns the HTTP method for this endpoint
	///
	/// Example: Method::GET
	fn method() -> Method;

	/// Returns the route name for URL reversal
	///
	/// Example: "get_user"
	fn name() -> &'static str;
}
