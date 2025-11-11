//! Core trait for parameter extraction

use crate::{ParamContext, ParamResult};
use async_trait::async_trait;
use reinhardt_http::Request;

/// Trait for types that can be extracted from a request
///
/// This is the core abstraction for parameter extraction.
/// All parameter types (Path, Query, Header, etc.) implement this trait.
#[async_trait]
pub trait FromRequest: Sized + Send {
	/// Extract this type from the request
	///
	/// # Arguments
	///
	/// * `req` - The HTTP request
	/// * `ctx` - Parameter context with extracted path params, etc.
	///
	/// # Returns
	///
	/// The extracted value or an error if extraction failed
	async fn from_request(req: &Request, ctx: &ParamContext) -> ParamResult<Self>;
}
