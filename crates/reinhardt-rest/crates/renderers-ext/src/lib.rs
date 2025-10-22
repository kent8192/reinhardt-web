//! Extended renderers functionality

pub mod admin_renderer;
pub mod csv_renderer;
pub mod documentation_renderer;
pub mod json;
pub mod openapi;
pub mod renderer;
pub mod schemajs_renderer;
pub mod static_html_renderer;
pub mod xml;
pub mod yaml_renderer;

// Re-export main types
pub use csv_renderer::CSVRenderer;
pub use openapi::OpenAPIRenderer;
pub use renderer::{RenderResult, Renderer, RendererContext};
pub use yaml_renderer::YAMLRenderer;

#[cfg(test)]
mod tests;
