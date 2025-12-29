//! URL patterns registration for compile-time discovery
//!
//! This module provides types for registering URL pattern functions
//! at compile time using the `inventory` crate. This allows the framework to
//! automatically discover and register routers without manual boilerplate in
//! management commands.
//!
//! # Important Constraints
//!
//! **Only one `#[routes]` function is allowed per project.** If multiple
//! functions are annotated with `#[routes]`, the linker will fail with a
//! "duplicate symbol" error for `__reinhardt_routes_registration_marker`.
//!
//! If you need to organize routes across multiple files, combine them in
//! a single root function:
//!
//! ```rust,ignore
//! // src/config/urls.rs
//! use reinhardt::prelude::*;
//! use reinhardt::routes;
//!
//! mod api;
//! mod web;
//!
//! #[routes]
//! pub fn routes() -> UnifiedRouter {
//!     UnifiedRouter::new()
//!         .mount("/api/", api::routes())  // Returns UnifiedRouter, not annotated with #[routes]
//!         .mount("/", web::routes())      // Returns UnifiedRouter, not annotated with #[routes]
//! }
//! ```
//!
//! # Architecture
//!
//! The URL patterns registration system follows the same pattern as other
//! compile-time registration systems in Reinhardt (DI, Signals, OpenAPI, ViewSets):
//!
//! 1. User code uses the `#[routes]` attribute macro on a function
//! 2. Macro generates an `inventory::submit!` call with a function pointer
//! 3. Framework code retrieves registrations via `inventory::iter::<UrlPatternsRegistration>()`
//! 4. Framework calls the registered functions and registers routers
//!
//! # Examples
//!
//! ```rust,ignore
//! // src/config/urls.rs
//! use reinhardt::prelude::*;
//! use reinhardt::routes;
//!
//! #[routes]
//! pub fn routes() -> UnifiedRouter {
//!     UnifiedRouter::new()
//!         .endpoint(views::index)
//!         .endpoint(views::about)
//! }
//! ```
//!
//! The `#[routes]` macro automatically handles `inventory` registration,
//! so you don't need any additional boilerplate code.

use crate::UnifiedRouter;
use std::sync::Arc;

/// URL patterns registration for compile-time discovery
///
/// This type is used with the `inventory` crate to register URL pattern
/// functions at compile time, allowing the framework to automatically
/// discover and register routers without manual boilerplate in management
/// commands like `runserver` or `check`.
///
/// # Fields
///
/// * `get_router` - Function pointer to get the router
///
/// # Implementation Details
///
/// This struct is collected by `inventory::collect!` and can be iterated
/// at runtime using `inventory::iter::<UrlPatternsRegistration>()`.
///
/// The framework automatically calls these functions in `execute_from_command_line()`
/// to register routers before executing management commands.
///
/// # Note
///
/// You typically don't create this struct directly. Instead, use the `#[routes]`
/// attribute macro which generates the registration code automatically.
#[derive(Clone)]
pub struct UrlPatternsRegistration {
	/// Function to get the router
	///
	/// This function returns an `Arc<UnifiedRouter>` with all application routes.
	/// The `#[routes]` macro wraps the user's function (which returns `UnifiedRouter`)
	/// in a closure that performs the `Arc::new()` call automatically.
	pub get_router: fn() -> Arc<UnifiedRouter>,
}

impl UrlPatternsRegistration {
	/// Create a new registration with the router factory function
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_routers::registration::UrlPatternsRegistration;
	/// use std::sync::Arc;
	///
	/// let registration = UrlPatternsRegistration::new(|| Arc::new(routes()));
	/// ```
	///
	/// # Note
	///
	/// You typically don't call this directly. Use the `#[routes]` macro instead.
	pub const fn new(get_router: fn() -> Arc<UnifiedRouter>) -> Self {
		Self { get_router }
	}
}

// Collect registrations for runtime iteration
inventory::collect!(UrlPatternsRegistration);
