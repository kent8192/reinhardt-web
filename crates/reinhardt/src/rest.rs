//! REST API module.
//!
//! This module provides REST API features including serializers, parsers,
//! pagination, filters, throttling, versioning, metadata, and content negotiation.
//!
//! # Examples
//!
//! ```rust,ignore
//! use reinhardt::rest::serializers::Serializer;
//! use reinhardt::rest::pagination::PageNumberPagination;
//! use reinhardt::rest::filters::FilterBackend;
//! ```

#[cfg(feature = "rest")]
pub use reinhardt_rest::*;
