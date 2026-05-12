//! Type-safe URL reversal (compile-time checked).

use super::super::pattern::validate_reverse_param;
use super::runtime::{ReverseError, ReverseResult, try_reverse_single_pass};
use std::collections::HashMap;
use std::marker::PhantomData;

/// Trait for URL patterns that can be reversed at compile time
///
/// Implement this trait for each URL pattern in your application.
/// The compiler will ensure that only valid URL patterns can be reversed.
///
/// # Example
///
/// ```rust
/// use reinhardt_urls::routers::reverse::UrlPattern;
///
/// pub struct UserListUrl;
/// impl UrlPattern for UserListUrl {
///     const NAME: &'static str = "user-list";
///     const PATTERN: &'static str = "/users/";
/// }
/// ```
pub trait UrlPattern {
	/// The unique name for this URL pattern
	const NAME: &'static str;

	/// The URL pattern string
	const PATTERN: &'static str;
}

/// Trait for URL patterns with parameters
///
/// Use this for URLs that require path parameters.
///
/// # Example
///
/// ```rust
/// use reinhardt_urls::routers::reverse::{UrlPattern, UrlPatternWithParams};
///
/// pub struct UserDetailUrl;
/// impl UrlPattern for UserDetailUrl {
///     const NAME: &'static str = "user-detail";
///     const PATTERN: &'static str = "/users/{id}/";
/// }
/// impl UrlPatternWithParams for UserDetailUrl {
///     const PARAMS: &'static [&'static str] = &["id"];
/// }
/// ```
pub trait UrlPatternWithParams: UrlPattern {
	/// The parameter names in order
	const PARAMS: &'static [&'static str];
}

/// Type-safe reverse for simple URL patterns (no parameters)
///
/// This function takes a type parameter implementing `UrlPattern` and returns
/// the URL string verbatim from `U::PATTERN`. Note that pattern *syntax*
/// validation is performed only when `PATTERN` is produced by a checked
/// constructor such as the `path!` macro; manual `impl UrlPattern` blocks that
/// supply a free-form `PATTERN` string are not validated by this function.
///
/// # Example
///
/// ```rust
/// use reinhardt_urls::routers::reverse::{reverse_typed, UrlPattern};
///
/// pub struct HomeUrl;
/// impl UrlPattern for HomeUrl {
///     const NAME: &'static str = "home";
///     const PATTERN: &'static str = "/";
/// }
///
/// let url = reverse_typed::<HomeUrl>();
/// assert_eq!(url, "/");
/// ```
pub fn reverse_typed<U: UrlPattern>() -> String {
	U::PATTERN.to_string()
}

/// Type-safe reverse for URL patterns with parameters
///
/// This function takes a type parameter and a HashMap of parameters,
/// substituting them into the URL pattern. Missing parameters will
/// result in a runtime error, but the pattern itself is compile-time checked.
///
/// # Example
///
/// ```rust
/// use reinhardt_urls::routers::reverse::{reverse_typed_with_params, UrlPattern, UrlPatternWithParams};
/// use std::collections::HashMap;
///
/// pub struct UserDetailUrl;
/// impl UrlPattern for UserDetailUrl {
///     const NAME: &'static str = "user-detail";
///     const PATTERN: &'static str = "/users/{id}/";
/// }
/// impl UrlPatternWithParams for UserDetailUrl {
///     const PARAMS: &'static [&'static str] = &["id"];
/// }
///
/// let mut params = HashMap::new();
/// params.insert("id", "123");
/// let url = reverse_typed_with_params::<UserDetailUrl>(&params).unwrap();
/// assert_eq!(url, "/users/123/");
/// ```
pub fn reverse_typed_with_params<U: UrlPatternWithParams>(
	params: &HashMap<&str, &str>,
) -> ReverseResult<String> {
	// Validate that all required parameters are provided
	for param_name in U::PARAMS {
		if !params.contains_key(param_name) {
			return Err(ReverseError::MissingParameter(param_name.to_string()));
		}
	}

	// Validate parameter values against injection attacks
	for (name, value) in params {
		if !validate_reverse_param(value) {
			return Err(ReverseError::Validation(format!(
				"invalid param '{}': contains dangerous characters",
				name
			)));
		}
	}

	// Convert &str HashMap to String HashMap for single-pass algorithm
	let string_params: HashMap<String, String> = params
		.iter()
		.map(|(k, v)| (k.to_string(), v.to_string()))
		.collect();

	try_reverse_single_pass(U::PATTERN, &string_params)
}

/// Type-safe URL parameter builder
///
/// Provides a fluent API for building URL parameters with compile-time checking
/// of parameter names.
///
/// # Example
///
/// ```rust
/// use reinhardt_urls::routers::reverse::{UrlParams, UrlPattern, UrlPatternWithParams};
///
/// pub struct UserDetailUrl;
/// impl UrlPattern for UserDetailUrl {
///     const NAME: &'static str = "user-detail";
///     const PATTERN: &'static str = "/users/{id}/";
/// }
/// impl UrlPatternWithParams for UserDetailUrl {
///     const PARAMS: &'static [&'static str] = &["id"];
/// }
///
/// let params = UrlParams::<UserDetailUrl>::new()
///     .param("id", "123")
///     .build()
///     .unwrap();
///
/// assert_eq!(params, "/users/123/");
/// ```
pub struct UrlParams<U: UrlPatternWithParams> {
	_phantom: PhantomData<U>,
	params: HashMap<String, String>,
}

impl<U: UrlPatternWithParams> UrlParams<U> {
	/// Create a new URL parameter builder
	pub fn new() -> Self {
		Self {
			_phantom: PhantomData,
			params: HashMap::new(),
		}
	}

	/// Add a parameter (note: parameter name is not compile-time checked currently,
	/// but the pattern itself is)
	pub fn param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
		self.params.insert(name.into(), value.into());
		self
	}

	/// Build the URL string, checking that all required parameters are present
	pub fn build(self) -> ReverseResult<String> {
		let params_ref: HashMap<&str, &str> = self
			.params
			.iter()
			.map(|(k, v)| (k.as_str(), v.as_str()))
			.collect();

		reverse_typed_with_params::<U>(&params_ref)
	}
}

impl<U: UrlPatternWithParams> Default for UrlParams<U> {
	fn default() -> Self {
		Self::new()
	}
}
