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
//! ## Planned Features
//!
//! - Async generator syntax integration when stable in Rust
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

// Development tools
#[cfg(feature = "dev-tools")]
pub mod visualization;

#[cfg(feature = "dev-tools")]
pub mod profiling;

#[cfg(feature = "dev-tools")]
pub mod advanced_cache;
