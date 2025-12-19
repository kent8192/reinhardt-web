//! API Client with Django QuerySet-like Interface
//!
//! This module provides a familiar Django-style interface for making API calls
//! from client-side WASM applications. It builds on top of Server Functions
//! to provide a high-level, type-safe API.
//!
//! ## Features
//!
//! - **QuerySet-like DSL**: Familiar methods like `filter()`, `exclude()`, `order_by()`
//! - **Type-safe**: Generic over model types with compile-time checking
//! - **Async-first**: All operations are async and return `Result`
//! - **CSRF Integration**: Automatic CSRF token injection
//!
//! ## Usage
//!
//! ```ignore
//! use reinhardt_pages::api::{ApiQuerySet, ApiModel};
//!
//! #[derive(ApiModel)]
//! #[api(endpoint = "/api/users/")]
//! struct User {
//!     id: i64,
//!     username: String,
//!     email: String,
//! }
//!
//! // QuerySet-like operations
//! let users = User::objects()
//!     .filter("is_active", true)
//!     .order_by(&["-created_at"])
//!     .limit(10)
//!     .all()
//!     .await?;
//!
//! let user = User::objects()
//!     .get(1)
//!     .await?;
//! ```

mod queryset;
mod registry;

pub use queryset::{ApiQuerySet, Filter, FilterOp};
pub use registry::ApiModel;
