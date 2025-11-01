//! Multipart form data handling
//!
//! This module provides support for `multipart/form-data` content type,
//! commonly used for file uploads.
//!
//! # Example
//!
//! ```rust,ignore
//! use reinhardt_params::Multipart;
//!
//! async fn upload_handler(mut multipart: Multipart) -> Result<(), Error> {
//!     while let Some(field) = multipart.next_field().await? {
//!         let name = field.name().unwrap_or("unknown");
//!         let data = field.bytes().await?;
//!         println!("Field {}: {} bytes", name, data.len());
//!     }
//!     Ok(())
//! }
//! ```

#[cfg(feature = "multipart")]
use crate::{FromRequest, ParamContext, ParamError, ParamResult};
#[cfg(feature = "multipart")]
use async_trait::async_trait;
#[cfg(feature = "multipart")]
use futures_util::future::ready;
#[cfg(feature = "multipart")]
use futures_util::stream::once;
#[cfg(feature = "multipart")]
use reinhardt_apps::Request;

#[cfg(feature = "multipart")]
pub use multer::{Field, Multipart as MulterMultipart};

/// Wrapper for multipart/form-data parsing
#[cfg(feature = "multipart")]
pub struct Multipart(pub MulterMultipart<'static>);

#[cfg(feature = "multipart")]
impl Multipart {
	/// Get the next field from the multipart stream
	///
	/// Returns `None` when all fields have been consumed.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_params::Multipart;
	/// # async fn example(mut multipart: Multipart) -> Result<(), Box<dyn std::error::Error>> {
	///
	// Iterate through all fields in the multipart request
	/// while let Some(field) = multipart.next_field().await? {
	///     let name = field.name().unwrap_or("unknown");
	///     println!("Processing field: {}", name);
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub async fn next_field(&mut self) -> Result<Option<Field<'static>>, multer::Error> {
		self.0.next_field().await
	}
}

#[cfg(feature = "multipart")]
#[async_trait]
impl FromRequest for Multipart {
	async fn from_request(req: &Request, _ctx: &ParamContext) -> ParamResult<Self> {
		// Extract boundary from Content-Type header
		let content_type = req
			.headers
			.get(http::header::CONTENT_TYPE)
			.and_then(|v| v.to_str().ok())
			.ok_or_else(|| ParamError::InvalidParameter {
				name: "content-type".to_string(),
				message: "Missing Content-Type header".to_string(),
			})?;

		// Parse boundary from content-type
		let boundary =
			multer::parse_boundary(content_type).map_err(|e| ParamError::InvalidParameter {
				name: "content-type".to_string(),
				message: format!("Failed to parse boundary: {}", e),
			})?;

		// Read body
		let body = req
			.read_body()
			.map_err(|e| ParamError::BodyError(format!("Failed to read body: {}", e)))?;

		// Convert Bytes to Stream (multer expects a Stream)
		let stream = once(ready(Ok::<_, std::io::Error>(body)));

		// Create multipart parser
		let multipart = MulterMultipart::new(stream, boundary);

		Ok(Multipart(multipart))
	}
}
