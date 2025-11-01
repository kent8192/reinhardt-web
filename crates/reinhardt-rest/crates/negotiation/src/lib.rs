//! Content negotiation for Reinhardt
//!
//! This crate provides DRF-style content negotiation for selecting
//! the appropriate renderer and parser based on Accept headers.
//!
//! ## Features
//!
//! - **Content negotiation**: Automatic media type selection based on Accept headers
//! - **Content-Type detection**: Automatic detection of request body format (JSON, XML, YAML, Form)
//! - **Language negotiation**: Support for Accept-Language header with quality factors
//! - **Encoding negotiation**: Support for Accept-Encoding header (Gzip, Brotli, Deflate, Identity)
//! - **Cache optimization**: Caching of negotiation results with TTL support

pub mod accept;
pub mod cache;
pub mod detector;
pub mod encoding;
pub mod language;
pub mod media_type;
pub mod negotiator;

pub use media_type::MediaType;
pub use negotiator::{
	BaseContentNegotiation, BaseNegotiator, ContentNegotiator, NegotiationError, RendererInfo,
};

/// Re-export commonly used types
pub mod prelude {
	pub use crate::cache::*;
	pub use crate::detector::*;
	pub use crate::encoding::*;
	pub use crate::language::*;
	pub use crate::media_type::*;
	pub use crate::negotiator::*;
}
