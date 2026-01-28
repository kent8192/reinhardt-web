//! Procedural macros for reinhardt-db NoSQL ODM
//!
//! This crate provides attribute macros for defining MongoDB documents:
//! - `#[document(...)]`: Defines a struct as a MongoDB document
//! - `#[field(...)]`: Adds metadata to document fields
//!
//! ## Example
//!
//! ```ignore
//! use reinhardt_db_macros::{document, field};
//!
//! #[document(collection = "users", backend = "mongodb")]
//! struct User {
//!     #[field(primary_key)]
//!     id: ObjectId,
//!
//!     #[field(required, unique)]
//!     email: String,
//!
//!     name: String,
//! }
//! ```

use proc_macro::TokenStream;

mod document;
mod field;

/// Defines a struct as a MongoDB document.
///
/// ## Attributes
///
/// - `collection` (required): The MongoDB collection name
/// - `backend` (required): Must be `"mongodb"`
/// - `database` (optional): The database name
///
/// ## Example
///
/// ```ignore
/// #[document(collection = "users", backend = "mongodb")]
/// struct User {
///     #[field(primary_key)]
///     id: ObjectId,
///     name: String,
/// }
/// ```
#[proc_macro_attribute]
pub fn document(attr: TokenStream, item: TokenStream) -> TokenStream {
	document::document_impl(attr, item)
}

/// Adds metadata to document fields.
///
/// ## Supported Attributes
///
/// - `primary_key`: Marks the field as the primary key
/// - `index`: Creates an index on the field
/// - `unique`: Creates a unique index on the field
/// - `required`: Makes the field required
/// - `default`: Specifies a default value
/// - `rename`: Renames the field in BSON
/// - `validate`: Specifies a validation function
/// - `min`: Minimum value (for numbers)
/// - `max`: Maximum value (for numbers)
/// - `references`: Marks the field as a foreign key reference
///
/// ## Example
///
/// ```ignore
/// #[field(required, unique, validate = "email")]
/// email: String,
/// ```
#[proc_macro_attribute]
pub fn field(attr: TokenStream, item: TokenStream) -> TokenStream {
	field::field_impl(attr, item)
}
