//! # Reinhardt Template System
//!
//! This crate provides a comprehensive template system for the Reinhardt framework,
//! including template engines, renderers, and runtime template processing.
//!
//! ## Features
//!
//! - **templates**: Template engine with Tera integration
//! - **templates-macros**: Template macros for compile-time validation
//! - **renderers**: Response renderers (JSON, XML, YAML, CSV, etc.)
//!
//! ## Re-exports
//!
//! This crate re-exports the following internal crates:
//!
//! - `reinhardt_templates`: Template engine functionality
//! - `reinhardt_templates_macros`: Procedural macros for templates
//! - `reinhardt_renderers`: Response renderers

#![doc(html_root_url = "https://docs.rs/reinhardt-template/0.1.0")]

// Re-export templates module
#[cfg(feature = "templates")]
pub use reinhardt_templates as templates;

// Re-export templates-macros module
#[cfg(feature = "templates-macros")]
pub use reinhardt_templates_macros as templates_macros;

// Re-export renderers module
#[cfg(feature = "renderers")]
pub use reinhardt_renderers as renderers;

// Convenience re-exports for common types
#[cfg(feature = "templates")]
pub use reinhardt_templates::TemplateError;

#[cfg(feature = "renderers")]
pub use reinhardt_renderers::{BrowsableAPIRenderer, JSONRenderer, TemplateHTMLRenderer};
