//! Renderer selection middleware utilities
//!
//! This module provides utilities for selecting the appropriate renderer
//! based on HTTP request information (Accept header, query parameters, URL format suffix).
//!
//! # Selection Priority
//!
//! 1. Format query parameter (e.g., `?format=json`)
//! 2. URL format suffix (e.g., `/api/users.json`)
//! 3. Accept header content negotiation
//! 4. Default renderer (first registered)
//!
//! # Examples
//!
//! ```
//! use reinhardt_renderers::{RendererRegistry, JSONRenderer};
//! use reinhardt_renderers::middleware::RendererSelector;
//!
//! let registry = RendererRegistry::new()
//!     .register(JSONRenderer::new());
//!
//! let selector = RendererSelector::new(&registry);
//!
//! // Select by format parameter
//! let renderer = selector.select(Some("json"), None, None).unwrap();
//!
//! // Select by Accept header
//! let renderer = selector.select(None, None, Some("application/json")).unwrap();
//! ```

use crate::format_suffix::extract_format_suffix;
use crate::renderer::{Renderer, RendererRegistry};
use reinhardt_exception::{Error, Result};

/// Helper for selecting renderers based on request information
///
/// This is a utility that can be used in middleware implementations
/// for various HTTP frameworks (axum, actix-web, etc.)
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::{RendererRegistry, JSONRenderer};
/// use reinhardt_renderers::middleware::RendererSelector;
///
/// let registry = RendererRegistry::new()
///     .register(JSONRenderer::new());
///
/// let selector = RendererSelector::new(&registry);
///
/// // Select by explicit format
/// let renderer = selector.select(Some("json"), None, None).unwrap();
///
/// // Select by URL path with suffix
/// let renderer = selector.select(None, Some("/api/users.json"), None).unwrap();
///
/// // Select by Accept header
/// let renderer = selector.select(None, None, Some("application/json")).unwrap();
/// ```
pub struct RendererSelector<'a> {
	registry: &'a RendererRegistry,
}

impl<'a> RendererSelector<'a> {
	/// Creates a new RendererSelector
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{RendererRegistry, JSONRenderer};
	/// use reinhardt_renderers::middleware::RendererSelector;
	///
	/// let registry = RendererRegistry::new()
	///     .register(JSONRenderer::new());
	///
	/// let selector = RendererSelector::new(&registry);
	/// ```
	pub fn new(registry: &'a RendererRegistry) -> Self {
		Self { registry }
	}

	/// Selects the appropriate renderer based on request information
	///
	/// # Priority
	///
	/// 1. `format_param` - explicit format query parameter
	/// 2. `url_path` - format suffix in URL path
	/// 3. `accept_header` - Accept header content negotiation
	/// 4. Default - first registered renderer
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{RendererRegistry, JSONRenderer};
	/// use reinhardt_renderers::middleware::RendererSelector;
	///
	/// let registry = RendererRegistry::new()
	///     .register(JSONRenderer::new());
	///
	/// let selector = RendererSelector::new(&registry);
	///
	/// // Priority 1: Format parameter takes precedence
	/// let renderer = selector.select(Some("json"), Some("/api/users.xml"), Some("application/xml"));
	/// // Returns JSON renderer because format parameter has highest priority
	///
	/// // Priority 2: URL suffix used when no format parameter
	/// let renderer = selector.select(None, Some("/api/users.xml"), Some("application/json"));
	/// // Returns XML renderer from URL suffix
	///
	/// // Priority 3: Accept header used when no format parameter or URL suffix
	/// let renderer = selector.select(None, Some("/api/users"), Some("application/xml"));
	/// // Returns XML renderer from Accept header
	/// ```
	pub fn select(
		&self,
		format_param: Option<&str>,
		url_path: Option<&str>,
		accept_header: Option<&str>,
	) -> Result<&dyn Renderer> {
		// Priority 1: Explicit format parameter
		if let Some(format) = format_param {
			return self
				.registry
				.get_renderer(Some(format))
				.ok_or_else(|| Error::Http(format!("No renderer for format: {}", format)));
		}

		// Priority 2: Format suffix in URL path
		if let Some(path) = url_path {
			let (_, format_suffix) = extract_format_suffix(path);
			if let Some(format) = format_suffix
				&& let Some(renderer) = self.registry.get_renderer(Some(format)) {
					return Ok(renderer);
				}
		}

		// Priority 3: Accept header negotiation
		if let Some(accept) = accept_header
			&& let Ok((renderer, _)) = self.registry.select_renderer(Some(accept)) {
				return Ok(renderer);
			}

		// Priority 4: Default to first registered renderer
		self.registry
			.get_renderer(None)
			.ok_or_else(|| Error::Http("No renderers registered".to_string()))
	}

	/// Selects renderer and extracts clean path (without format suffix)
	///
	/// Returns tuple of (selected renderer, clean path, format)
	/// where clean path has the format suffix removed if present.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{RendererRegistry, JSONRenderer};
	/// use reinhardt_renderers::middleware::RendererSelector;
	///
	/// let registry = RendererRegistry::new()
	///     .register(JSONRenderer::new());
	///
	/// let selector = RendererSelector::new(&registry);
	///
	/// let (renderer, clean_path, format) = selector
	///     .select_with_clean_path(None, Some("/api/users.json"), None)
	///     .unwrap();
	///
	/// assert_eq!(clean_path, "/api/users");
	/// assert_eq!(format, Some("json".to_string()));
	/// ```
	pub fn select_with_clean_path(
		&self,
		format_param: Option<&str>,
		url_path: Option<&str>,
		accept_header: Option<&str>,
	) -> Result<(&dyn Renderer, String, Option<String>)> {
		let (clean_path, format_from_suffix) = if let Some(path) = url_path {
			let (clean, suffix) = extract_format_suffix(path);
			(clean.to_string(), suffix.map(|s| s.to_string()))
		} else {
			(String::new(), None)
		};

		// Determine effective format
		let effective_format = format_param
			.map(|s| s.to_string())
			.or(format_from_suffix.clone());

		let renderer = self.select(format_param, url_path, accept_header)?;

		Ok((renderer, clean_path, effective_format))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{JSONRenderer, XMLRenderer};

	#[test]
	fn test_selector_format_param_priority() {
		let registry = RendererRegistry::new()
			.register(JSONRenderer::new())
			.register(XMLRenderer::new());

		let selector = RendererSelector::new(&registry);

		// Format parameter should take precedence
		let renderer = selector
			.select(
				Some("json"),
				Some("/api/users.xml"),
				Some("application/xml"),
			)
			.unwrap();

		assert_eq!(renderer.format(), Some("json"));
	}

	#[test]
	fn test_selector_url_suffix_priority() {
		let registry = RendererRegistry::new()
			.register(JSONRenderer::new())
			.register(XMLRenderer::new());

		let selector = RendererSelector::new(&registry);

		// URL suffix should be used when no format parameter
		let renderer = selector
			.select(None, Some("/api/users.xml"), Some("application/json"))
			.unwrap();

		assert_eq!(renderer.format(), Some("xml"));
	}

	#[test]
	fn test_selector_accept_header_priority() {
		let registry = RendererRegistry::new()
			.register(JSONRenderer::new())
			.register(XMLRenderer::new());

		let selector = RendererSelector::new(&registry);

		// Accept header should be used when no format parameter or URL suffix
		let renderer = selector
			.select(None, Some("/api/users"), Some("application/xml"))
			.unwrap();

		assert_eq!(renderer.format(), Some("xml"));
	}

	#[test]
	fn test_selector_default_renderer() {
		let registry = RendererRegistry::new()
			.register(JSONRenderer::new())
			.register(XMLRenderer::new());

		let selector = RendererSelector::new(&registry);

		// Should default to first registered renderer
		let renderer = selector.select(None, None, None).unwrap();

		assert_eq!(renderer.format(), Some("json"));
	}

	#[test]
	fn test_selector_with_clean_path() {
		let registry = RendererRegistry::new().register(JSONRenderer::new());

		let selector = RendererSelector::new(&registry);

		let (renderer, clean_path, format) = selector
			.select_with_clean_path(None, Some("/api/users.json"), None)
			.unwrap();

		assert_eq!(renderer.format(), Some("json"));
		assert_eq!(clean_path, "/api/users");
		assert_eq!(format, Some("json".to_string()));
	}

	#[test]
	fn test_selector_error_no_renderers() {
		let registry = RendererRegistry::new();
		let selector = RendererSelector::new(&registry);

		let result = selector.select(None, None, None);
		assert!(result.is_err());
	}

	#[test]
	fn test_selector_error_unknown_format() {
		let registry = RendererRegistry::new().register(JSONRenderer::new());

		let selector = RendererSelector::new(&registry);

		let result = selector.select(Some("unknown"), None, None);
		assert!(result.is_err());
	}
}
