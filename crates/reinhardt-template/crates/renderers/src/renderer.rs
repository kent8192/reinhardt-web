//! Core renderer traits and types

use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_exception::{Error, Result};
use reinhardt_negotiation::{ContentNegotiator, MediaType};
use serde_json::Value;
use std::collections::HashMap;

pub type RenderResult<T> = Result<T>;

/// Context information for rendering operations
///
/// Stores information about the HTTP request, view, and additional
/// metadata that renderers can use during rendering.
#[derive(Debug, Clone, Default)]
pub struct RendererContext {
	/// HTTP method and path
	pub request: Option<(String, String)>,
	/// View name and description
	pub view: Option<(String, String)>,
	/// Additional metadata
	pub extra: HashMap<String, String>,
	/// Accept header from the request
	pub accept_header: Option<String>,
	/// Format query parameter
	pub format_param: Option<String>,
}

impl RendererContext {
	/// Creates a new empty RendererContext
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::RendererContext;
	///
	/// let context = RendererContext::new();
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the HTTP request information
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::RendererContext;
	///
	/// let context = RendererContext::new()
	///     .with_request("GET", "/api/items");
	/// ```
	pub fn with_request(mut self, method: &str, path: &str) -> Self {
		self.request = Some((method.to_string(), path.to_string()));
		self
	}

	/// Sets the view information
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::RendererContext;
	///
	/// let context = RendererContext::new()
	///     .with_view("ItemList", "Returns a list of items");
	/// ```
	pub fn with_view(mut self, name: &str, description: &str) -> Self {
		self.view = Some((name.to_string(), description.to_string()));
		self
	}

	/// Adds extra metadata
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::RendererContext;
	///
	/// let context = RendererContext::new()
	///     .with_extra("api_version", "v1")
	///     .with_extra("authenticated", "true");
	/// ```
	pub fn with_extra(mut self, key: &str, value: &str) -> Self {
		self.extra.insert(key.to_string(), value.to_string());
		self
	}

	/// Sets the Accept header from the HTTP request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::RendererContext;
	///
	/// let context = RendererContext::new()
	///     .with_accept_header("application/json");
	/// ```
	pub fn with_accept_header(mut self, accept: &str) -> Self {
		self.accept_header = Some(accept.to_string());
		self
	}

	/// Sets the format query parameter
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::RendererContext;
	///
	/// let context = RendererContext::new()
	///     .with_format_param("json");
	/// ```
	pub fn with_format_param(mut self, format: &str) -> Self {
		self.format_param = Some(format.to_string());
		self
	}
}

#[async_trait]
pub trait Renderer: Send + Sync {
	/// Returns the primary media type for this renderer
	fn media_type(&self) -> String {
		self.media_types().first().cloned().unwrap_or_default()
	}

	/// Returns all supported media types
	fn media_types(&self) -> Vec<String>;

	/// Renders the data to bytes
	async fn render(&self, data: &Value, context: Option<&RendererContext>) -> RenderResult<Bytes>;

	/// Returns the format identifier (e.g., "json", "xml")
	fn format(&self) -> Option<&str> {
		None
	}
}

/// Registry for managing multiple renderers
#[derive(Default)]
pub struct RendererRegistry {
	renderers: Vec<Box<dyn Renderer>>,
	negotiator: ContentNegotiator,
}

impl RendererRegistry {
	/// Creates a new renderer registry
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{RendererRegistry, JSONRenderer};
	///
	/// let registry = RendererRegistry::new()
	///     .register(JSONRenderer::new());
	/// ```
	pub fn new() -> Self {
		Self {
			renderers: Vec::new(),
			negotiator: ContentNegotiator::new(),
		}
	}

	/// Registers a renderer in the registry
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{RendererRegistry, JSONRenderer};
	///
	/// let registry = RendererRegistry::new()
	///     .register(JSONRenderer::new());
	/// ```
	pub fn register<R: Renderer + 'static>(mut self, renderer: R) -> Self {
		self.renderers.push(Box::new(renderer));
		self
	}

	/// Gets a renderer by format string
	pub fn get_renderer(&self, format: Option<&str>) -> Option<&dyn Renderer> {
		if let Some(fmt) = format {
			self.renderers
				.iter()
				.find(|r| r.format() == Some(fmt))
				.map(|r| r.as_ref())
		} else {
			self.renderers.first().map(|r| r.as_ref())
		}
	}

	/// Gets a renderer by media type using content negotiation
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{RendererRegistry, JSONRenderer};
	///
	/// let registry = RendererRegistry::new()
	///     .register(JSONRenderer::new());
	///
	/// let renderer = registry.get_renderer_by_media_type("application/json");
	/// assert!(renderer.is_some());
	/// ```
	pub fn get_renderer_by_media_type(&self, media_type: &str) -> Option<&dyn Renderer> {
		self.renderers
			.iter()
			.find(|r| r.media_types().iter().any(|mt| mt == media_type))
			.map(|r| r.as_ref())
	}

	/// Selects the best renderer based on Accept header
	///
	/// Returns the selected renderer and the accepted media type string.
	/// Returns an error if no suitable renderer is found (406 Not Acceptable).
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{RendererRegistry, JSONRenderer};
	///
	/// let registry = RendererRegistry::new()
	///     .register(JSONRenderer::new());
	///
	/// let result = registry.select_renderer(Some("application/json"));
	/// assert!(result.is_ok());
	/// ```
	pub fn select_renderer(
		&self,
		accept_header: Option<&str>,
	) -> RenderResult<(&dyn Renderer, String)> {
		if self.renderers.is_empty() {
			return Err(Error::Http("No renderers registered".to_string()));
		}

		// Build list of available media types
		let available_media_types: Vec<MediaType> = self
			.renderers
			.iter()
			.flat_map(|r| {
				r.media_types().into_iter().filter_map(|mt| {
					let parts: Vec<&str> = mt.split('/').collect();
					if parts.len() == 2 {
						Some(MediaType::new(parts[0], parts[1]))
					} else {
						None
					}
				})
			})
			.collect();

		// Use content negotiation to select the best match
		let result = self
			.negotiator
			.select_renderer(accept_header, &available_media_types);

		match result {
			Ok((media_type, media_type_str)) => {
				// Find the renderer that supports this media type
				let renderer = self
					.get_renderer_by_media_type(&media_type.to_string())
					.ok_or_else(|| {
						Error::Http(format!("No renderer for media type: {}", media_type))
					})?;
				Ok((renderer, media_type_str))
			}
			Err(_) => Err(Error::Http(
				"Could not satisfy Accept header - 406 Not Acceptable".to_string(),
			)),
		}
	}

	/// Renders data with content negotiation support
	///
	/// This method supports:
	/// - Format query parameter (e.g., ?format=json)
	/// - Accept header negotiation
	/// - Default to first renderer if no preferences specified
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{RendererRegistry, JSONRenderer, RendererContext};
	/// use serde_json::json;
	///
	/// # use tokio;
	/// # #[tokio::main]
	/// # async fn main() {
	/// let registry = RendererRegistry::new()
	///     .register(JSONRenderer::new());
	///
	/// let data = json!({"message": "hello"});
	/// let context = RendererContext::new()
	///     .with_accept_header("application/json");
	///
	/// let result = registry.render(&data, None, Some(&context)).await;
	/// assert!(result.is_ok());
	/// # }
	/// ```
	pub async fn render(
		&self,
		data: &Value,
		format: Option<&str>,
		context: Option<&RendererContext>,
	) -> RenderResult<(Bytes, String)> {
		// Priority:
		// 1. Explicit format parameter
		// 2. Format from context
		// 3. Accept header negotiation from context
		// 4. First registered renderer

		let selected_format = format.or_else(|| context.and_then(|c| c.format_param.as_deref()));

		let renderer = if let Some(fmt) = selected_format {
			// Use format-based selection
			self.get_renderer(Some(fmt))
				.ok_or_else(|| Error::Http(format!("No renderer for format: {}", fmt)))?
		} else if let Some(ctx) = context {
			// Try Accept header negotiation
			if let Some(accept) = &ctx.accept_header {
				let (renderer, _) = self.select_renderer(Some(accept))?;
				renderer
			} else {
				// Fall back to first renderer
				self.renderers
					.first()
					.ok_or_else(|| Error::Http("No renderers registered".to_string()))?
					.as_ref()
			}
		} else {
			// No context, use first renderer
			self.renderers
				.first()
				.ok_or_else(|| Error::Http("No renderers registered".to_string()))?
				.as_ref()
		};

		let bytes = renderer.render(data, context).await?;
		let content_type = renderer.media_type();

		Ok((bytes, content_type))
	}
}
