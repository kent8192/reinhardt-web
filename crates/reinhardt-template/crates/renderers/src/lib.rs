//! # Reinhardt Renderers
//!
//! Response renderers for the Reinhardt framework, inspired by Django REST Framework.
//!
//! ## Renderers
//!
//! - **JSONRenderer**: Render responses as JSON
//! - **BrowsableAPIRenderer**: HTML self-documenting API interface (re-exported from reinhardt-browsable-api)
//! - **XMLRenderer**: Render responses as XML
//! - **YAMLRenderer**: Render responses as YAML
//! - **CSVRenderer**: Render responses as CSV
//! - **OpenAPIRenderer**: Generate OpenAPI 3.0 specifications
//! - **AdminRenderer**: Django-like admin interface renderer
//! - **StaticHTMLRenderer**: Static HTML content renderer
//! - **DocumentationRenderer**: Render API documentation from OpenAPI schemas
//! - **SchemaJSRenderer**: Render OpenAPI schemas as JavaScript
//!
//! ## Renderer Selection
//!
//! The framework provides automatic renderer selection based on:
//!
//! 1. **Format query parameter** (e.g., `?format=json`)
//! 2. **URL format suffix** (e.g., `/api/users.json`)
//! 3. **Accept header** content negotiation with quality values (q-factor)
//! 4. **Default renderer** (first registered)
//!
//! ## Example - Basic Usage
//!
//! ```rust,ignore
//! use reinhardt_renderers::{JSONRenderer, Renderer};
//!
//! let renderer = JSONRenderer::new();
//! let response = renderer.render(&data, None).await?;
//! ```
//!
//! ## Example - Renderer Registry
//!
//! ```rust
//! use reinhardt_renderers::{RendererRegistry, JSONRenderer, XMLRenderer};
//! use reinhardt_renderers::RendererContext;
//! use serde_json::json;
//!
//! # use tokio;
//! # #[tokio::main]
//! # async fn main() {
//! // Create a registry and register renderers
//! let registry = RendererRegistry::new()
//!     .register(JSONRenderer::new())
//!     .register(XMLRenderer::new());
//!
//! let data = json!({"message": "hello"});
//!
//! // Render with automatic selection based on Accept header
//! let context = RendererContext::new()
//!     .with_accept_header("application/json");
//!
//! let (bytes, content_type) = registry.render(&data, None, Some(&context)).await.unwrap();
//! # }
//! ```
//!
//! ## Example - Renderer Selection with Middleware
//!
//! ```rust
//! use reinhardt_renderers::{RendererRegistry, JSONRenderer, XMLRenderer};
//! use reinhardt_renderers::RendererSelector;
//!
//! // Create registry
//! let registry = RendererRegistry::new()
//!     .register(JSONRenderer::new())
//!     .register(XMLRenderer::new());
//!
//! let selector = RendererSelector::new(&registry);
//!
//! // Priority 1: Format parameter takes precedence
//! let renderer = selector.select(
//!     Some("json"),                    // format parameter
//!     Some("/api/users.xml"),          // URL path with suffix
//!     Some("application/xml"),         // Accept header
//! ).unwrap();
//!
//! // Returns JSON renderer because format parameter has highest priority
//! assert_eq!(renderer.format(), Some("json"));
//! ```
//!
//! ## Example - Format Suffix Extraction
//!
//! ```rust
//! use reinhardt_renderers::format_suffix::{extract_format_suffix, get_media_type_for_format};
//!
//! // Extract format suffix from URL path
//! let (clean_path, format) = extract_format_suffix("/api/users.json");
//! assert_eq!(clean_path, "/api/users");
//! assert_eq!(format, Some("json"));
//!
//! // Get media type for format
//! let media_type = get_media_type_for_format("json");
//! assert_eq!(media_type, Some("application/json"));
//! ```
//!
//! ## Advanced Features
//!
//! ### Renderer Chaining
//!
//! Chain multiple renderers to transform data through multiple stages:
//!
//! ```rust
//! use reinhardt_renderers::{RendererChain, JSONRenderer, Renderer};
//! use serde_json::json;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let renderer_chain = RendererChain::new()
//!     .pipe(JSONRenderer::new());
//!
//! // Data flows through the renderer pipeline
//! let data = json!({"message": "hello"});
//! let result = renderer_chain.render(&data, None).await.unwrap();
//! # }
//! ```
//!
//! ### Response Caching
//!
//! Cache rendered responses to avoid redundant rendering of identical data:
//!
//! ```rust
//! use reinhardt_renderers::{CachedRenderer, JSONRenderer, CacheConfig, Renderer};
//! use std::time::Duration;
//! use serde_json::json;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let cached_renderer = CachedRenderer::new(
//!     JSONRenderer::new(),
//!     CacheConfig::new()
//!         .with_ttl(Duration::from_secs(300))
//!         .with_max_capacity(1000)
//! );
//!
//! let data = json!({"message": "hello"});
//!
//! // First call renders and caches
//! let result1 = cached_renderer.render(&data, None).await.unwrap();
//!
//! // Second call returns cached result (no re-rendering)
//! let result2 = cached_renderer.render(&data, None).await.unwrap();
//! # }
//! ```
//!
//! ### Streaming Support
//!
//! Stream large responses incrementally instead of buffering entire response:
//!
//! ```rust
//! use reinhardt_renderers::streaming::{StreamingJSONRenderer, StreamingRenderer};
//! use serde_json::json;
//! use futures::StreamExt;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let streaming_renderer = StreamingJSONRenderer::new();
//! let large_dataset = json!([{"id": 1}, {"id": 2}, {"id": 3}]);
//!
//! // Returns Stream<Item = Result<Bytes, Error>> instead of Bytes
//! let mut stream = streaming_renderer.render_stream(&large_dataset, None).await.unwrap();
//!
//! // Stream can be consumed incrementally
//! while let Some(chunk) = stream.next().await {
//!     let bytes = chunk.unwrap();
//!     // send_to_client(bytes).await;
//! }
//! # }
//! ```
//!
//! Available streaming renderers:
//! - **StreamingJSONRenderer**: Streams JSON arrays element by element
//! - **StreamingCSVRenderer**: Streams CSV rows incrementally
//!
//! ### Compression Support
//!
//! Automatic response compression with multiple algorithms:
//!
//! ```rust
//! use reinhardt_renderers::{CompressionRenderer, CompressionAlgorithm, JSONRenderer, Renderer, RendererContext};
//! use serde_json::json;
//!
//! # #[tokio::main]
//! # async fn main() {
//! // Or use content negotiation
//! let compressed_renderer = CompressionRenderer::new(
//!     JSONRenderer::new(),
//!     vec![
//!         CompressionAlgorithm::Brotli { quality: 4 },
//!         CompressionAlgorithm::Gzip { level: 6 },
//!         CompressionAlgorithm::Deflate,
//!     ]
//! );
//!
//! let data = json!({"message": "hello"});
//! let context = RendererContext::new()
//!     .with_extra("accept_encoding", "gzip");
//!
//! // Automatically selects best compression based on Accept-Encoding header
//! let bytes = compressed_renderer
//!     .render(&data, Some(&context))
//!     .await.unwrap();
//! # }
//! ```
//!
//! ## Runtime Template Rendering (Phase 3)
//!
//! Tera integration for flexible runtime template rendering:
//!
//! ```rust
//! use reinhardt_renderers::TeraRenderer;
//! use serde_json::json;
//!
//! let renderer = TeraRenderer::new();
//! let context = json!({
//!     "name": "Alice",
//!     "email": "alice@example.com",
//!     "age": 25
//! });
//!
//! let html = renderer.render_template("user.tpl", &context).unwrap();
//! ```
//!
//! Choose the right template strategy:
//!
//! ```rust
//! use reinhardt_renderers::strategy::{TemplateStrategy, TemplateStrategySelector, TemplateSource};
//!
//! // Static templates → CompileTime (embedded)
//! let source = TemplateSource::Static("user.html");
//! let strategy = TemplateStrategySelector::select(&source);
//! assert_eq!(strategy, TemplateStrategy::CompileTime);
//!
//! // Dynamic templates → Runtime (flexible)
//! let source = TemplateSource::Dynamic("<h1>{{ title }}</h1>".to_string());
//! let strategy = TemplateStrategySelector::select(&source);
//! assert_eq!(strategy, TemplateStrategy::Runtime);
//! ```

pub mod admin_renderer;
pub mod strategy;
pub mod tera_renderer;
pub mod cached;
pub mod chain;
pub mod compression;
pub mod csv_renderer;
pub mod documentation_renderer;
pub mod format_suffix;
pub mod json;
pub mod middleware;
pub mod openapi;
pub mod renderer;
pub mod schemajs_renderer;
pub mod static_html_renderer;
pub mod streaming;
pub mod template_html_renderer;
pub mod xml;
pub mod yaml_renderer;

#[cfg(test)]
mod tests;

pub use admin_renderer::AdminRenderer;
pub use tera_renderer::{
    Post, PostListTemplate, TeraRenderer, UserData, UserListTemplate, UserTemplate,
};
pub use cached::{CacheConfig, CachedRenderer};
pub use chain::RendererChain;
pub use compression::{CompressionAlgorithm, CompressionRenderer};
pub use csv_renderer::CSVRenderer;
pub use documentation_renderer::DocumentationRenderer;
pub use json::JSONRenderer;
pub use middleware::RendererSelector;
pub use openapi::OpenAPIRenderer;
pub use renderer::{RenderResult, Renderer, RendererContext, RendererRegistry};
pub use schemajs_renderer::SchemaJSRenderer;
pub use static_html_renderer::StaticHTMLRenderer;
pub use strategy::{TemplateSource, TemplateStrategy, TemplateStrategySelector};
pub use streaming::{
    StreamingConfig, StreamingCSVRenderer, StreamingJSONRenderer, StreamingRenderer,
};
pub use template_html_renderer::TemplateHTMLRenderer;
pub use xml::XMLRenderer;
pub use yaml_renderer::YAMLRenderer;

// Re-export from specialized crates
pub use reinhardt_browsable_api::BrowsableApiRenderer as BrowsableAPIRenderer;
