//! Type-safe field lookup system with compile-time validation
//!
//! This module provides a field lookup API with full type safety enforced at compile time.
//!
//! # Examples
//!
//! ```rust,ignore
//! use reinhardt_orm::prelude::*;
//!
//! #[derive(Model, QueryFields)]
//! struct User {
//!     id: i64,
//!     email: String,
//!     age: i32,
//!     created_at: DateTime,
//! }
//!
//! // Type-safe queries
//! QuerySet::<User>::new()
//!     .filter(User::email().lower().contains("example.com"))
//!     .filter(User::age().gte(18))
//!     .filter(User::created_at().year().eq(2025));
//!
//! // Compile errors for invalid operations
//! // User::age().contains(18);  // ERROR: contains() only available for String
//! // User::email().year();       // ERROR: year() only available for DateTime
//! ```

mod compiler;
mod field;
mod lookup;
mod traits;

pub use compiler::QueryFieldCompiler;
pub use field::Field;
pub use lookup::{Lookup, LookupType, LookupValue};
pub use traits::{Comparable, Date, DateTime, DateTimeType, NumericType, StringType};
