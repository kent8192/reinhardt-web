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
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_renderers::{JSONRenderer, Renderer};
//!
//! let renderer = JSONRenderer::new();
//! let response = renderer.render(&data, None).await?;
//! ```

pub mod admin_renderer;
pub mod csv_renderer;
pub mod documentation_renderer;
pub mod json;
pub mod openapi;
pub mod renderer;
pub mod schemajs_renderer;
pub mod static_html_renderer;
pub mod template_html_renderer;
pub mod xml;
pub mod yaml_renderer;

#[cfg(test)]
mod tests;

pub use admin_renderer::AdminRenderer;
pub use csv_renderer::CSVRenderer;
pub use documentation_renderer::DocumentationRenderer;
pub use json::JSONRenderer;
pub use openapi::OpenAPIRenderer;
pub use renderer::{RenderResult, Renderer, RendererContext, RendererRegistry};
pub use schemajs_renderer::SchemaJSRenderer;
pub use static_html_renderer::StaticHTMLRenderer;
pub use template_html_renderer::TemplateHTMLRenderer;
pub use xml::XMLRenderer;
pub use yaml_renderer::YAMLRenderer;

// Re-export from specialized crates
pub use reinhardt_browsable_api::BrowsableApiRenderer as BrowsableAPIRenderer;
