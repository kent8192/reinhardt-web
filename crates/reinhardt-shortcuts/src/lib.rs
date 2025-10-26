//!
//! Convenient shortcut functions for common Reinhardt operations.
//!
//! Equivalent to Django's `django.shortcuts` module.
//!
//! ## Examples
//!
//! ```rust,ignore
//! use reinhardt_shortcuts::{render_template, redirect, get_object_or_404};
//!
//! // Render a template with context
//! let response = render_template(&request, "template.html", context)?;
//!
//! // Redirect to a URL
//! let response = redirect("/users/")?;
//!
//! // Get object or return 404 (requires "database" feature)
//! let user = get_object_or_404::<User>(id).await?;
//! ```
//!
//! ## Planned Features
//! TODO: Custom error pages (404, 500, etc.)
//! TODO: Error page templates
//! TODO: Debug error pages for development
//! TODO: Full Askama template engine integration for variable substitution
//! TODO: Template inheritance and includes
//! TODO: Custom template filters and tags

pub mod get_or_404;
pub mod redirect;
pub mod render;

// ORM integration (feature-gated)
#[cfg(feature = "database")]
pub mod orm;

// Template integration (feature-gated)
#[cfg(feature = "templates")]
pub mod template;

// Template caching for performance (feature-gated)
#[cfg(feature = "templates")]
pub mod template_cache;

// Template inheritance support with Tera (feature-gated)
#[cfg(feature = "templates")]
pub mod template_inheritance;

// Custom error pages (feature-gated)
#[cfg(feature = "templates")]
pub mod error_pages;

// Re-export core functions
pub use get_or_404::{
    exists_or_404_response, get_list_or_404_response, get_or_404_response, GetError,
};
pub use redirect::{redirect, redirect_permanent};
pub use render::{render_html, render_json, render_json_pretty, render_text};

// Re-export ORM functions (feature-gated)
#[cfg(feature = "database")]
pub use orm::{get_list_or_404, get_object_or_404};

// Re-export template functions (feature-gated)
#[cfg(feature = "templates")]
pub use template::{render_template, render_to_response};

// Re-export error page functions (feature-gated)
#[cfg(feature = "templates")]
pub use error_pages::{
    bad_request, page_not_found, permission_denied, render_error_page, server_error,
};
