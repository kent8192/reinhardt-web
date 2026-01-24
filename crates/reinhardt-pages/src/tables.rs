//! Table utilities for reinhardt-pages (django-tables2 equivalent)
//!
//! This module provides declarative table definition with various column types,
//! sorting, pagination, filtering, and export functionality.
//!
//! # Features
//!
//! - Declarative table definition with `#[derive(Table)]`
//! - Multiple column types: `Column`, `LinkColumn`, `BooleanColumn`, `DateTimeColumn`, etc.
//! - Sorting support with URL parameters (`?sort=field`, `?sort=-field`)
//! - Pagination with configurable page size
//! - Export to CSV, JSON, Excel, YAML
//! - Filtering with URL parameters
//! - Integration with `page!` macro
//!
//! # Example
//!
//! ```rust,ignore
//! use reinhardt_pages::tables::*;
//!
//! #[derive(Table)]
//! struct UserTable {
//!     #[column(name = "ID")]
//!     id: Column<i32>,
//!     #[column(name = "Name")]
//!     name: Column<String>,
//!     #[column(name = "Email", link = "/users/{id}")]
//!     email: LinkColumn<String>,
//!     #[column(name = "Active")]
//!     is_active: BooleanColumn,
//! }
//! ```

pub mod column;
pub mod columns;
pub mod export;
pub mod filtering;
pub mod pagination;
pub mod sorting;
pub mod table;

// Re-exports for convenience
pub use column::Column as ColumnTrait;
pub use columns::*;
pub use export::{ExportFormat, Exportable};
pub use filtering::Filterable;
pub use pagination::Pagination;
pub use sorting::{SortDirection, Sortable};
pub use table::Table;
