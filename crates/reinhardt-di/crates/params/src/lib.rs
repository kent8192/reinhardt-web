//! # Reinhardt Parameter Extraction
//!
//! FastAPI-inspired parameter extraction system.
//!
//! ## Features
//!
//! - **Path Parameters**: Extract from URL path
//! - **Query Parameters**: Extract from query string
//! - **Headers**: Extract from request headers
//! - **Cookies**: Extract from cookies
//! - **Body**: Extract from request body
//! - **Type-safe**: Full compile-time type checking
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_params::{Path, Query, Json};
//!
//! #[endpoint(GET "/users/{id}")]
//! async fn get_user(
//!     id: Path<i64>,
//!     filter: Query<UserFilter>,
//!     body: Json<UpdateUser>,
//! ) -> Result<User> {
//!     // id.0 is the extracted i64
//!     // filter.0 is the extracted UserFilter
//!     // body.0 is the extracted UpdateUser
//! }
//! ```

pub mod body;
pub mod cookie;
pub mod cookie_named;
pub mod extract;
pub mod form;
pub mod header;
pub mod header_named;
pub mod json;
#[cfg(feature = "multipart")]
pub mod multipart;
pub mod path;
pub mod query;
pub mod validation;

use reinhardt_apps::Error as CoreError;
use std::any::TypeId;
use std::collections::HashMap;
use thiserror::Error;

pub use body::Body;
pub use cookie::{Cookie, CookieStruct};
pub use cookie_named::{CookieName, CookieNamed, CsrfToken, SessionId};
pub use extract::FromRequest;
pub use form::Form;
pub use header::{Header, HeaderStruct};
pub use header_named::{Authorization, ContentType, HeaderName, HeaderNamed};
pub use json::Json;
#[cfg(feature = "multipart")]
pub use multipart::Multipart;
pub use path::{Path, PathStruct};
pub use query::Query;
#[cfg(feature = "validation")]
pub use validation::Validated;
pub use validation::{
	ValidatedForm, ValidatedPath, ValidatedQuery, ValidationConstraints, WithValidation,
};

#[derive(Debug, Error)]
pub enum ParamError {
	#[error("Missing required parameter: {0}")]
	MissingParameter(String),

	#[error("Invalid parameter value for '{name}': {message}")]
	InvalidParameter { name: String, message: String },

	#[error("Failed to parse parameter '{name}': {source}")]
	ParseError {
		name: String,
		#[source]
		source: Box<dyn std::error::Error + Send + Sync>,
	},

	#[error("Deserialization error: {0}")]
	DeserializationError(#[from] serde_json::Error),

	#[error("URL encoding error: {0}")]
	UrlEncodingError(#[from] serde_urlencoded::de::Error),

	#[error("Request body error: {0}")]
	BodyError(String),

	#[cfg(feature = "validation")]
	#[error("Validation error for '{name}': {message}")]
	ValidationError { name: String, message: String },
}

impl From<ParamError> for CoreError {
	fn from(err: ParamError) -> Self {
		CoreError::Validation(err.to_string())
	}
}

pub type ParamResult<T> = std::result::Result<T, ParamError>;

/// Context for parameter extraction
pub struct ParamContext {
	/// Path parameters extracted from the URL
	pub path_params: std::collections::HashMap<String, String>,
	/// Header name registry keyed by value type
	header_names: HashMap<TypeId, &'static str>,
	/// Cookie name registry keyed by value type
	cookie_names: HashMap<TypeId, &'static str>,
}

impl ParamContext {
	/// Create a new empty ParamContext
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_params::ParamContext;
	///
	/// let ctx = ParamContext::new();
	/// assert_eq!(ctx.path_params.len(), 0);
	/// ```
	pub fn new() -> Self {
		Self {
			path_params: std::collections::HashMap::new(),
			header_names: HashMap::new(),
			cookie_names: HashMap::new(),
		}
	}
	/// Create a ParamContext with pre-populated path parameters
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_params::ParamContext;
	/// use std::collections::HashMap;
	///
	/// let mut params = HashMap::new();
	/// params.insert("id".to_string(), "42".to_string());
	/// params.insert("name".to_string(), "test".to_string());
	///
	/// let ctx = ParamContext::with_path_params(params);
	/// assert_eq!(ctx.get_path_param("id"), Some("42"));
	/// assert_eq!(ctx.get_path_param("name"), Some("test"));
	/// ```
	pub fn with_path_params(path_params: std::collections::HashMap<String, String>) -> Self {
		Self {
			path_params,
			header_names: HashMap::new(),
			cookie_names: HashMap::new(),
		}
	}
	/// Get a path parameter by name
	///
	/// Returns `None` if the parameter doesn't exist.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_params::ParamContext;
	/// use std::collections::HashMap;
	///
	/// let mut params = HashMap::new();
	/// params.insert("user_id".to_string(), "123".to_string());
	///
	/// let ctx = ParamContext::with_path_params(params);
	/// assert_eq!(ctx.get_path_param("user_id"), Some("123"));
	/// assert_eq!(ctx.get_path_param("missing"), None);
	/// ```
	pub fn get_path_param(&self, name: &str) -> Option<&str> {
		self.path_params.get(name).map(|s| s.as_str())
	}

	/// Register a header name for the given value type `T`
	pub fn set_header_name<T: 'static>(&mut self, name: &'static str) {
		self.header_names.insert(TypeId::of::<T>(), name);
	}

	/// Get a registered header name for value type `T`
	pub fn get_header_name<T: 'static>(&self) -> Option<&'static str> {
		self.header_names.get(&TypeId::of::<T>()).copied()
	}

	/// Register a cookie name for the given value type `T`
	pub fn set_cookie_name<T: 'static>(&mut self, name: &'static str) {
		self.cookie_names.insert(TypeId::of::<T>(), name);
	}

	/// Get a registered cookie name for value type `T`
	pub fn get_cookie_name<T: 'static>(&self) -> Option<&'static str> {
		self.cookie_names.get(&TypeId::of::<T>()).copied()
	}
}

impl Default for ParamContext {
	fn default() -> Self {
		Self::new()
	}
}
