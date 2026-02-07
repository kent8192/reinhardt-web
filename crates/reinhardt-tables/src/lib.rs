//! Data table rendering utilities for Reinhardt
//!
//! This crate provides Django-tables2 equivalent functionality for Reinhardt,
//! enabling declarative table definitions with sorting, pagination, filtering,
//! and export capabilities.
//!
//! # Features
//!
//! - **Table Definition**: Declarative table definition using traits and builders
//! - **Column Types**: Multiple column types (Link, Boolean, DateTime, etc.)
//! - **Sorting**: URL parameter-based sorting (`?sort=field`)
//! - **Pagination**: Page navigation with `?page=N`
//! - **Filtering**: Column filtering with URL parameters
//! - **Export**: CSV and JSON export (requires `export` feature)
//! - **Integration**: Seamless integration with `reinhardt-pages` (requires `pages-integration` feature)
//!
//! # Architecture
//!
//! ```mermaid
//! graph TD
//!     A[Table] --> B[Columns]
//!     A --> C[Rows]
//!     A --> D[Sort Config]
//!     A --> E[Pagination]
//!     A --> F[Filters]
//!     B --> G[Column Types]
//!     G --> H[LinkColumn]
//!     G --> I[BooleanColumn]
//!     G --> J[DateTimeColumn]
//!     A --> K[Export]
//!     K --> L[CSV]
//!     K --> M[JSON]
//! ```
//!
//! # Example
//!
//! ```rust
//! use reinhardt_tables::{Table, Column};
//!
//! struct User {
//!     id: i32,
//!     name: String,
//!     active: bool,
//! }
//!
//! // Basic table usage will be demonstrated once core types are implemented
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

pub mod column;
pub mod error;
pub mod table;

// Re-exports for convenience
pub use column::Column;
pub use error::{Result, TableError};
pub use table::Table;
