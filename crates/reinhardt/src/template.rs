//! Template and rendering module.
//!
//! This module provides template engine, template macros, and renderers.
//!
//! # Examples
//!
//! ```rust,ignore
//! use reinhardt::template::templates::Template;
//! use reinhardt::template::renderers::JSONRenderer;
//! ```

#[cfg(feature = "templates")]
pub use reinhardt_template::*;
