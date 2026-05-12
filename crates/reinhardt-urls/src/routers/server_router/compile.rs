//! Route compilation and validation for [`ServerRouter`].
//!
//! These methods build the per-method `matchit` routers and surface any
//! pattern errors at startup. Compilation is lazy and recovers from
//! `RwLock` poisoning to avoid cascade failures.

use super::ServerRouter;
use super::handlers::ViewSetHandler;
use super::types::RouteHandler;
use hyper::Method;
use reinhardt_views::viewsets::Action;
use std::borrow::Cow;
use std::sync::{Arc, PoisonError};

impl ServerRouter {
	/// Compile all routes into matchit routers.
	///
	/// This should be called after all routes have been registered.
	/// It converts patterns like "/users/{id}" to matchit format.
	///
	/// Returns a list of route compilation errors (if any). Empty list means
	/// all routes compiled successfully. RwLock poisoning is recovered from
	/// via `PoisonError::into_inner` to prevent cascade failures.
	pub(crate) fn compile_routes(&self) -> Vec<String> {
		// Check if already compiled (read lock, recovers from poisoning)
		if *self
			.routes_compiled
			.read()
			.unwrap_or_else(PoisonError::into_inner)
		{
			return Vec::new();
		}

		let mut errors = Vec::new();

		// Compile function routes
		for func_route in &self.functions {
			let route_handler = RouteHandler {
				handler: func_route.handler.clone(),
				middleware: func_route.middleware.clone(),
			};

			// Strip prefix from route path to avoid double-prefix matching.
			// Routes may be registered with absolute paths that already include the prefix
			// (e.g., server functions register as "/api/server_fn/login"). Since resolve()
			// strips the prefix from incoming request paths before matching against matchit,
			// we must also strip the prefix here during compilation.
			let route_path_owned = Self::strip_prefix_normalized(&self.prefix, &func_route.path)
				.unwrap_or_else(|| Cow::Borrowed(&func_route.path));
			let route_path: &str = &route_path_owned;

			// matchit uses {name} format which matches our pattern
			let router_lock = match func_route.method {
				Method::GET => &self.get_router,
				Method::POST => &self.post_router,
				Method::PUT => &self.put_router,
				Method::DELETE => &self.delete_router,
				Method::PATCH => &self.patch_router,
				Method::HEAD => &self.head_router,
				Method::OPTIONS => &self.options_router,
				_ => &self.get_router,
			};
			if let Err(e) = router_lock
				.write()
				.unwrap_or_else(PoisonError::into_inner)
				.insert(route_path, route_handler)
			{
				errors.push(format!(
					"Failed to compile route '{}' ({}): {}",
					func_route.path, func_route.method, e
				));
			}
		}

		// Compile view routes (views handle all methods internally)
		for view_route in &self.views {
			let route_handler = RouteHandler {
				handler: view_route.handler.clone(),
				middleware: view_route.middleware.clone(),
			};

			// Strip prefix from route path (same reason as function routes above)
			let route_path_owned = Self::strip_prefix_normalized(&self.prefix, &view_route.path)
				.unwrap_or_else(|| Cow::Borrowed(&view_route.path));
			let route_path: &str = &route_path_owned;

			// Register view for all common HTTP methods
			for router_lock in &[
				&self.get_router,
				&self.post_router,
				&self.put_router,
				&self.delete_router,
				&self.patch_router,
			] {
				if let Err(e) = router_lock
					.write()
					.unwrap_or_else(PoisonError::into_inner)
					.insert(route_path, route_handler.clone())
				{
					errors.push(format!(
						"Failed to compile view route '{}': {}",
						view_route.path, e
					));
				}
			}
		}

		// Compile raw routes (routes handle all methods internally)
		for route in &self.routes {
			let route_handler = RouteHandler {
				handler: route.handler_arc(),
				middleware: route.middleware.clone(),
			};

			// Strip prefix from route path (same reason as function routes above)
			let route_path_owned = Self::strip_prefix_normalized(&self.prefix, &route.path)
				.unwrap_or_else(|| Cow::Borrowed(&route.path));
			let route_path: &str = &route_path_owned;

			// Register raw route for all common HTTP methods
			for router_lock in &[
				&self.get_router,
				&self.post_router,
				&self.put_router,
				&self.delete_router,
				&self.patch_router,
			] {
				if let Err(e) = router_lock
					.write()
					.unwrap_or_else(PoisonError::into_inner)
					.insert(route_path, route_handler.clone())
				{
					errors.push(format!(
						"Failed to compile raw route '{}': {}",
						route.path, e
					));
				}
			}
		}

		// Compile ViewSet routes
		// ViewSet base_path must NOT include self.prefix because resolve() strips
		// the prefix from incoming request paths before matching against matchit.
		for (prefix, viewset) in &self.viewsets {
			let base_path = format!("/{}", prefix.trim_start_matches('/'));

			// Collection route: GET /prefix/ (list), POST /prefix/ (create)
			let collection_path = format!("{}/", base_path.trim_end_matches('/'));

			// List action (GET)
			let list_handler = RouteHandler {
				handler: Arc::new(ViewSetHandler {
					viewset: viewset.clone(),
					action: Action::list(),
				}),
				middleware: Vec::new(),
			};
			if let Err(e) = self
				.get_router
				.write()
				.unwrap_or_else(PoisonError::into_inner)
				.insert(&collection_path, list_handler)
			{
				errors.push(format!(
					"Failed to compile ViewSet list route '{}': {}",
					collection_path, e
				));
			}

			// Create action (POST)
			let create_handler = RouteHandler {
				handler: Arc::new(ViewSetHandler {
					viewset: viewset.clone(),
					action: Action::create(),
				}),
				middleware: Vec::new(),
			};
			if let Err(e) = self
				.post_router
				.write()
				.unwrap_or_else(PoisonError::into_inner)
				.insert(&collection_path, create_handler)
			{
				errors.push(format!(
					"Failed to compile ViewSet create route '{}': {}",
					collection_path, e
				));
			}

			// Detail routes: GET/PUT/DELETE /prefix/{id}/
			let lookup_field = viewset.get_lookup_field();
			let detail_path = format!("{}/{{{}}}/", base_path.trim_end_matches('/'), lookup_field);

			// Retrieve action (GET)
			let retrieve_handler = RouteHandler {
				handler: Arc::new(ViewSetHandler {
					viewset: viewset.clone(),
					action: Action::retrieve(),
				}),
				middleware: Vec::new(),
			};
			if let Err(e) = self
				.get_router
				.write()
				.unwrap_or_else(PoisonError::into_inner)
				.insert(&detail_path, retrieve_handler)
			{
				errors.push(format!(
					"Failed to compile ViewSet retrieve route '{}': {}",
					detail_path, e
				));
			}

			// Update action (PUT)
			let update_handler = RouteHandler {
				handler: Arc::new(ViewSetHandler {
					viewset: viewset.clone(),
					action: Action::update(),
				}),
				middleware: Vec::new(),
			};
			if let Err(e) = self
				.put_router
				.write()
				.unwrap_or_else(PoisonError::into_inner)
				.insert(&detail_path, update_handler)
			{
				errors.push(format!(
					"Failed to compile ViewSet update route '{}': {}",
					detail_path, e
				));
			}

			// Destroy action (DELETE)
			let destroy_handler = RouteHandler {
				handler: Arc::new(ViewSetHandler {
					viewset: viewset.clone(),
					action: Action::destroy(),
				}),
				middleware: Vec::new(),
			};
			if let Err(e) = self
				.delete_router
				.write()
				.unwrap_or_else(PoisonError::into_inner)
				.insert(&detail_path, destroy_handler)
			{
				errors.push(format!(
					"Failed to compile ViewSet destroy route '{}': {}",
					detail_path, e
				));
			}
		}

		// Mark routes as compiled
		*self
			.routes_compiled
			.write()
			.unwrap_or_else(PoisonError::into_inner) = true;

		errors
	}

	/// Validate all routes by compiling them and returning any errors.
	///
	/// Call this at application startup to detect invalid route patterns early.
	/// Returns `Ok(())` if all routes compiled successfully, or `Err` with
	/// a list of compilation error messages.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::ServerRouter;
	/// use hyper::Method;
	/// # use reinhardt_http::{Request, Response, Result};
	///
	/// # async fn handler(_req: Request) -> Result<Response> { Ok(Response::ok()) }
	/// let router = ServerRouter::new()
	///     .function("/users/{id}", Method::GET, handler);
	///
	/// // Validate routes at startup
	/// assert!(router.validate_routes().is_ok());
	/// ```
	pub fn validate_routes(&self) -> std::result::Result<(), Vec<String>> {
		let mut errors = self.compile_routes();
		if let Err(name_errors) = self.validate_route_names() {
			errors.extend(name_errors);
		}
		if errors.is_empty() {
			Ok(())
		} else {
			Err(errors)
		}
	}

	/// Validate that no duplicate route names exist among all collected routes.
	///
	/// Returns `Ok(())` if all names are unique, or `Err(errors)` with details
	/// about each duplicate.
	pub fn validate_route_names(&self) -> std::result::Result<(), Vec<String>> {
		let registrations = self.collect_routes_recursive(None, "");
		let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
		let mut errors = Vec::new();
		for (name, path) in registrations {
			if let Some(existing_path) = seen.insert(name.clone(), path.clone()) {
				errors.push(format!(
					"Duplicate route name '{}': path '{}' conflicts with existing path '{}'",
					name, path, existing_path
				));
			}
		}
		if errors.is_empty() {
			Ok(())
		} else {
			Err(errors)
		}
	}
}
