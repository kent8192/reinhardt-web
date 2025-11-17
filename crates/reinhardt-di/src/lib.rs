//! # Reinhardt Dependency Injection
//!
//! FastAPI-inspired dependency injection system for Reinhardt.
//!
//! ## Features
//!
//! - **Type-safe**: Full compile-time type checking
//! - **Async-first**: Built for async/await
//! - **Scoped**: Request-scoped and singleton dependencies
//! - **Composable**: Dependencies can depend on other dependencies
//! - **Cache**: Automatic caching within request scope
//!
//! ## Development Tools (dev-tools feature)
//!
//! When the `dev-tools` feature is enabled, additional debugging and profiling tools are available:
//!
//! - **Visualization**: Generate dependency graphs in DOT format for Graphviz
//! - **Profiling**: Track dependency resolution performance and identify bottlenecks
//! - **Advanced Caching**: LRU and TTL-based caching strategies
//!
//! ## Generator Support (generator feature) ✅
//!
//! Generator-based dependency resolution for lazy, streaming dependency injection.
//!
//! **Note**: Uses `genawaiter` crate as a workaround for unstable native async yield.
//! Will be migrated to native syntax when Rust stabilizes async generators.
//!
//! ```rust,ignore
//! #[cfg(feature = "generator")]
//! use reinhardt_di::generator::DependencyGenerator;
//!
//! #[cfg(feature = "generator")]
//! let gen = DependencyGenerator::new(|co| async move {
//!     let db = resolve_database().await;
//!     co.yield_(db).await;
//!
//!     let cache = resolve_cache().await;
//!     co.yield_(cache).await;
//! });
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_di::{Depends, Injectable};
//!
//! // Define a dependency
//! struct Database {
//!     pool: DbPool,
//! }
//!
//! #[async_trait]
//! impl Injectable for Database {
//!     async fn inject(ctx: &InjectionContext) -> Result<Self> {
//!         Ok(Database {
//!             pool: get_pool().await?,
//!         })
//!     }
//! }
//!
//! // Use in endpoint
//! #[endpoint(GET "/users")]
//! async fn list_users(
//!     db: Depends<Database>,
//! ) -> Result<Vec<User>> {
//!     db.query("SELECT * FROM users").await
//! }
//! ```
//!
//! ## InjectionContext Construction
//!
//! InjectionContext is constructed using the builder pattern with a required singleton scope:
//!
//! ```rust
//! use reinhardt_di::{InjectionContext, SingletonScope};
//! use std::sync::Arc;
//!
//! // Create singleton scope
//! let singleton = Arc::new(SingletonScope::new());
//!
//! // Build injection context with singleton scope
//! let ctx = InjectionContext::builder(singleton).build();
//! ```
//!
//! Optional request and param context can be added:
//!
//! ```rust
//! use reinhardt_di::{InjectionContext, SingletonScope};
//! use reinhardt_http::Request;
//! use std::sync::Arc;
//!
//! let singleton = Arc::new(SingletonScope::new());
//!
//! // Create a dummy request for demonstration
//! let request = Request::builder()
//!     .method(hyper::Method::GET)
//!     .uri("/")
//!     .version(hyper::Version::HTTP_11)
//!     .headers(hyper::HeaderMap::new())
//!     .body(bytes::Bytes::new())
//!     .build()
//!     .unwrap();
//!
//! let ctx = InjectionContext::builder(singleton)
//!     .with_request(request)
//!     .build();
//! ```
//!
//! ## Development Tools Example
//!
//! ```rust,ignore
//! #[cfg(feature = "dev-tools")]
//! use reinhardt_di::{visualization::DependencyGraph, profiling::DependencyProfiler};
//!
//! #[cfg(feature = "dev-tools")]
//! fn visualize_dependencies() {
//!     let mut graph = DependencyGraph::new();
//!     graph.add_node("Database", "singleton");
//!     graph.add_node("UserService", "request");
//!     graph.add_dependency("UserService", "Database");
//!
//!     println!("{}", graph.to_dot());
//! }
//!
//! #[cfg(feature = "dev-tools")]
//! fn profile_resolution() {
//!     let mut profiler = DependencyProfiler::new();
//!     profiler.start_resolve("Database");
//!     // ... perform resolution ...
//!     profiler.end_resolve("Database");
//!
//!     let report = profiler.generate_report();
//!     println!("{}", report.to_string());
//! }
//! ```

// Re-export DI core
pub use di::*;

#[cfg(feature = "params")]
pub use reinhardt_params as params;

// Generator support
#[cfg(feature = "generator")]
pub mod generator;

// Development tools
#[cfg(feature = "dev-tools")]
pub mod visualization;

#[cfg(feature = "dev-tools")]
pub mod profiling;

#[cfg(feature = "dev-tools")]
pub mod advanced_cache;
