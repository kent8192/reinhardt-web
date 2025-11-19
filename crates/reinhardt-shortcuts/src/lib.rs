//!
//! Convenient shortcut functions for common Reinhardt operations.
//!
//! Equivalent to Django's `django.shortcuts` module.
//!
//! ## Core Shortcuts
//!
//! ### Rendering Templates (Tera)
//!
//! ```rust,ignore
//! use reinhardt_shortcuts::render_template;
//! use std::collections::HashMap;
//!
//! // Render a Tera template with context
//! let mut context = HashMap::new();
//! context.insert("title", "Welcome");
//! context.insert("user", user.name());
//!
//! let response = render_template(&request, "index.html", context)?;
//! ```
//!
//! ### Custom Error Pages
//!
//! ```rust,ignore
//! use reinhardt_shortcuts::{page_not_found, server_error, render_debug_error_page};
//! use std::collections::HashMap;
//!
//! // Return a 404 error page
//! return Err(page_not_found(&request, None));
//!
//! // Return a 500 error page with context
//! let mut context = HashMap::new();
//! context.insert("error_details", "Database connection failed");
//! return Err(server_error(&request, Some(context)));
//!
//! // Debug error page (development only)
//! let debug_response = render_debug_error_page(
//!     &request,
//!     500,
//!     "Detailed error message",
//!     Some(context)
//! );
//! ```
//!
//! ### Redirects
//!
//! ```rust,ignore
//! use reinhardt_shortcuts::{redirect, redirect_permanent};
//!
//! // Temporary redirect (302)
//! let response = redirect("/users/")?;
//!
//! // Permanent redirect (301)
//! let response = redirect_permanent("/new-location/")?;
//! ```
//!
//! ### Database Shortcuts
//!
//! ```rust,ignore
//! use reinhardt_shortcuts::get_object_or_404;
//!
//! // Get object or return 404 (requires "database" feature)
//! let user = get_object_or_404::<User>(id).await?;
//! ```
//!
//! ## Template Features
//!
//! ### Custom Filters
//!
//! Available Tera filters:
//!
//! ```jinja
//! {{ long_text | truncate_chars(length=50, suffix="...") }}
//! {{ number | intcomma }}  // 1234567 → 1,234,567
//! {{ count }} item{{ count | pluralize }}
//! {{ value | default(value="N/A") }}
//! {{ field_html | add_class(class="form-control") }}
//! ```
//!
//! ### Custom Functions/Tags
//!
//! Available Tera functions:
//!
//! ```jinja
//! {% for i in range(start=0, end=10) %}
//!   {{ i }}
//! {% endfor %}
//!
//! {{ now(format="%Y-%m-%d %H:%M:%S") }}
//!
//! {% for item in items %}
//!   <div class="{{ cycle(values=["odd", "even"], index=loop.index0) }}">
//!     {{ item }}
//!   </div>
//! {% endfor %}
//!
//! <img src="{{ static(path="images/logo.png") }}">
//! <a href="{{ url(name="user_profile", id=user.id) }}">Profile</a>
//! ```
//!
//! ## Implemented Features
//!
//! - ✅ Custom error pages (404, 500, 403, 400, etc.)
//! - ✅ Error page templates with Tera
//! - ✅ Debug error pages for development environments
//! - ✅ Template inheritance and includes (Tera-based)
//! - ✅ Custom template filters (truncate_chars, intcomma, pluralize, default, add_class)
//! - ✅ Custom template functions/tags (range, now, cycle, static, url)

pub mod context;
pub mod get_or_404;
pub mod redirect;
pub mod render;
pub mod url;

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

// Custom Tera filters (feature-gated)
#[cfg(feature = "templates")]
pub mod tera_filters;

// Custom Tera functions/tags (feature-gated)
#[cfg(feature = "templates")]
pub mod tera_functions;

// Re-export core functions
pub use context::TemplateContext;
pub use get_or_404::{
	GetError, exists_or_404_response, get_list_or_404_response, get_or_404_response,
};
pub use redirect::{redirect, redirect_permanent, redirect_permanent_to, redirect_to};
pub use render::{render_html, render_json, render_json_pretty, render_text};
pub use url::{Url, UrlError};

// Re-export ORM functions (feature-gated)
#[cfg(feature = "database")]
pub use orm::{get_list_or_404, get_object_or_404};

// Re-export template functions (feature-gated)
#[cfg(feature = "templates")]
pub use template::{
	render_template, render_template_with_context, render_to_response,
	render_to_response_with_context,
};

// Re-export error page functions (feature-gated)
#[cfg(feature = "templates")]
pub use error_pages::{
	ErrorPageBuilder, bad_request, page_not_found, permission_denied, render_debug_error_page,
	render_error_page, server_error,
};

// Re-export template cache types (feature-gated)
#[cfg(feature = "templates")]
pub use template_cache::{EvictedEntry, TemplateCacheError};
