//! Header parameter extraction

use async_trait::async_trait;
use reinhardt_http::Request;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::ops::Deref;

use crate::{ParamContext, ParamError, ParamResult, extract::FromRequest};

/// Extract a value from request headers
pub struct Header<T>(pub T);

impl<T> Header<T> {
	/// Unwrap the Header and return the inner value
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_params::Header;
	///
	/// let header = Header(String::from("application/json"));
	/// let inner = header.into_inner();
	/// assert_eq!(inner, "application/json");
	/// ```
	pub fn into_inner(self) -> T {
		self.0
	}
}

impl<T> Deref for Header<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T: Debug> Debug for Header<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

/// HeaderStruct extracts multiple headers into a struct
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Deserialize)]
/// struct MyHeaders {
///     #[serde(rename = "x-api-key")]
///     api_key: String,
///
///     #[serde(rename = "user-agent")]
///     user_agent: Option<String>,
/// }
///
/// async fn handler(headers: HeaderStruct<MyHeaders>) {
///     let api_key = headers.api_key;
/// }
/// ```
pub struct HeaderStruct<T>(pub T);

impl<T> HeaderStruct<T> {
	/// Unwrap the HeaderStruct and return the inner value
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_params::HeaderStruct;
	/// use serde::Deserialize;
	///
	/// #[derive(Deserialize, Debug, PartialEq)]
	/// struct MyHeaders {
	///     content_type: String,
	/// }
	///
	/// let headers = HeaderStruct(MyHeaders {
	///     content_type: "text/html".to_string(),
	/// });
	/// let inner = headers.into_inner();
	/// assert_eq!(inner.content_type, "text/html");
	/// ```
	pub fn into_inner(self) -> T {
		self.0
	}
}

impl<T> Deref for HeaderStruct<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T: Debug> Debug for HeaderStruct<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

/// Convert headers to a map for deserialization
/// Header names are converted to lowercase
fn headers_to_map(req: &Request) -> HashMap<String, String> {
	let mut result = HashMap::new();

	for (name, value) in req.headers.iter() {
		if let Ok(value_str) = value.to_str() {
			result.insert(name.as_str().to_lowercase(), value_str.to_string());
		}
	}

	result
}

#[async_trait]
impl<T> FromRequest for HeaderStruct<T>
where
	T: DeserializeOwned + Send,
{
	async fn from_request(req: &Request, _ctx: &ParamContext) -> ParamResult<Self> {
		let headers_map = headers_to_map(req);

		// Use serde_urlencoded for proper string-to-type deserialization
		// This handles type coercion naturally (e.g., "123" -> i64)
		let encoded =
			serde_urlencoded::to_string(&headers_map).map_err(|e| ParamError::ParseError {
				name: "headers".to_string(),
				source: Box::new(e),
			})?;

		serde_urlencoded::from_str(&encoded)
			.map(HeaderStruct)
			.map_err(|e| ParamError::ParseError {
				name: "headers".to_string(),
				source: Box::new(e),
			})
	}
}

#[async_trait]
impl FromRequest for Header<String> {
	async fn from_request(req: &Request, ctx: &ParamContext) -> ParamResult<Self> {
		let name = ctx.get_header_name::<String>().ok_or_else(|| {
			ParamError::MissingParameter(
				"Header name not specified in ParamContext for this type".to_string(),
			)
		})?;

		let value = req
			.headers
			.get(name)
			.and_then(|v| v.to_str().ok())
			.ok_or_else(|| ParamError::MissingParameter(name.to_string()))?;

		Ok(Header(value.to_string()))
	}
}

#[async_trait]
impl FromRequest for Header<Option<String>> {
	async fn from_request(req: &Request, ctx: &ParamContext) -> ParamResult<Self> {
		let name = match ctx.get_header_name::<String>() {
			Some(n) => n,
			None => return Ok(Header(None)),
		};
		let maybe = req.headers.get(name).and_then(|v| v.to_str().ok());
		Ok(Header(maybe.map(|s| s.to_string())))
	}
}

// Implement WithValidation trait for Header
#[cfg(feature = "validation")]
impl<T> crate::validation::WithValidation for Header<T> {}
