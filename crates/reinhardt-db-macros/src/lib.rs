//! # Reinhardt DB Macros
//!
//! Procedural macros for the Reinhardt database layer.
//!
//! This crate provides attribute macros for:
//! - ORM models: `#[model(...)]`
//! - NoSQL documents: `#[document(...)]`
//! - Field attributes: `#[field(...)]`
//!
//! ## Feature Flags
//!
//! - `orm` - Enable ORM model macros (SQL)
//! - `nosql` - Enable NoSQL document macros (MongoDB, etc.)
//!
//! ## NoSQL ODM Example
//!
//! ```rust,ignore
//! use reinhardt_db_macros::{document, field};
//! use bson::oid::ObjectId;
//!
//! #[document(collection = "users", backend = "mongodb")]
//! struct User {
//!     #[field(primary_key)]
//!     id: ObjectId,
//!     #[field(required, unique)]
//!     email: String,
//!     name: String,
//! }
//! ```

use proc_macro::TokenStream;

/// Document macro for NoSQL ODM
///
/// Generates the `Document` trait implementation for structs representing
/// NoSQL database documents (e.g., MongoDB collections).
///
/// ## Attributes
///
/// - `collection` - Collection/table name (required)
/// - `backend` - Database backend: "mongodb" (required)
/// - `database` - Database name (optional, defaults to "default")
///
/// ## Example
///
/// ```rust,ignore
/// use reinhardt_db_macros::{document, field};
/// use bson::oid::ObjectId;
///
/// #[document(collection = "users", backend = "mongodb")]
/// struct User {
///     #[field(primary_key)]
///     id: ObjectId,
///     email: String,
/// }
/// ```
#[proc_macro_attribute]
pub fn document(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // TODO: Implement document macro expansion
    // For now, just return the input unchanged
    item
}

/// Field attribute for document/model fields
///
/// Provides metadata and validation for individual fields.
///
/// ## Attributes
///
/// - `primary_key` - Mark as primary key
/// - `required` - Field is required (non-null)
/// - `unique` - Field must be unique
/// - `index` - Create index on this field
/// - `default` - Default value expression
/// - `rename` - Rename field in database
/// - `validate` - Custom validation function
/// - `min` / `max` - Numeric range constraints
/// - `references` - Foreign key reference
///
/// ## Example
///
/// ```rust,ignore
/// use reinhardt_db_macros::{document, field};
///
/// #[document(collection = "users", backend = "mongodb")]
/// struct User {
///     #[field(primary_key)]
///     id: ObjectId,
///     #[field(required, unique)]
///     email: String,
///     #[field(default = "Anonymous")]
///     name: String,
/// }
/// ```
#[proc_macro_attribute]
pub fn field(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // TODO: Implement field attribute processing
    // For now, just return the input unchanged
    item
}

/// Model derive macro for ORM
///
/// Automatically derives the `Model` trait for SQL ORM models.
///
/// ## Example
///
/// ```rust,ignore
/// use reinhardt_db_macros::Model;
///
/// #[derive(Model)]
/// struct User {
///     id: i32,
///     email: String,
/// }
/// ```
#[proc_macro_derive(Model)]
pub fn derive_model(_input: TokenStream) -> TokenStream {
    // TODO: Implement Model derive macro
    // For now, return empty implementation
    TokenStream::new()
}
