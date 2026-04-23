#![warn(missing_docs)]

//! # Reinhardt OpenAPI Router
//!
//! OpenAPI router wrapper that automatically adds documentation endpoints.
//!
//! ## Overview
//!
//! This crate provides a router wrapper that intercepts requests to OpenAPI
//! documentation paths and serves them from memory, delegating all other
//! requests to the wrapped handler.
//!
//! ## Example
//!
//! Define your application routes in a `routes()` function returning a
//! `UnifiedRouter`, then wrap the router with `OpenApiRouter` at the server
//! setup level — outside of `routes()` itself.
//!
//! ```rust,ignore
//! use reinhardt_openapi::OpenApiRouter;
//! use reinhardt_urls::routers::UnifiedRouter;
//!
//! // Define routes using the project-standard routes() function.
//! // The #[cfg_attr(native, routes(standalone))] attribute registers
//! // this function as the application entry point in native builds.
//! #[cfg_attr(native, routes(standalone))]
//! pub fn routes() -> UnifiedRouter {
//!     UnifiedRouter::new()
//!     // ... mount app routes here ...
//! }
//!
//! // In server setup or tests, wrap the routes() output with OpenApiRouter.
//! // Note: in production, the `runserver` command applies this automatically.
//! fn start_server() -> Result<(), Box<dyn std::error::Error>> {
//!     let handler = OpenApiRouter::wrap(routes())?;
//!
//!     // handler now serves:
//!     // - /openapi.json (OpenAPI spec)
//!     // - /docs (Swagger UI)
//!     // - /redoc (Redoc UI)
//!     Ok(())
//! }
//! ```
//!
//! ## Separation Rationale
//!
//! This crate exists separately from `reinhardt-rest` to break a circular
//! dependency chain:
//!
//! ```text
//! reinhardt-urls → reinhardt-views → reinhardt-rest → reinhardt-urls (cycle!)
//! ```
//!
//! By placing `OpenApiRouter` in its own crate that depends on both
//! `reinhardt-urls` and `reinhardt-rest`, we avoid this cycle:
//!
//! ```text
//! reinhardt-openapi
//!     ├── reinhardt-urls (Route, Router trait)
//!     └── reinhardt-rest (generate_openapi_schema, SwaggerUI, RedocUI)
//! ```

mod router_wrapper;

pub use reinhardt_rest::openapi::SchemaError;
pub use router_wrapper::AuthGuard;
pub use router_wrapper::OpenApiRouter;
