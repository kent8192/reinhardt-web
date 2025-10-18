//! # Reinhardt Shortcuts
//!
//! Convenient shortcut functions for common Reinhardt operations.
//!
//! Equivalent to Django's `django.shortcuts` module.
//!
//! ## Examples
//!
//! ```rust,ignore
//! use reinhardt_shortcuts::{render, redirect, get_object_or_404};
//!
//! // Render a template with context
//! let response = render(request, "template.html", context)?;
//!
//! // Redirect to a URL
//! let response = redirect("/users/")?;
//!
//! // Get object or return 404
//! let user = get_object_or_404(User::objects(), id)?;
//! ```

// TODO: Implement shortcut modules
// pub mod render;
// pub mod redirect;
// pub mod get_or_404;

// pub use render::render;
// pub use redirect::{redirect, redirect_to};

// #[cfg(feature = "database")]
// pub use get_or_404::{get_object_or_404, get_list_or_404};
