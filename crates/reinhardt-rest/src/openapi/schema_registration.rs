//! Compile-time schema registration infrastructure
//!
//! This module provides the infrastructure for automatically registering OpenAPI schemas
//! at compile time using the `inventory` crate. When types are annotated with
//! `#[derive(Schema)]`, the macro automatically generates a `SchemaRegistration` entry
//! that is collected at compile time.
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::openapi::{Schema, ToSchema};
//!
//! #[derive(Schema)]
//! pub struct User {
//!     pub id: i64,
//!     pub name: String,
//! }
//!
//! // The macro automatically generates:
//! // inventory::submit! {
//! //     SchemaRegistration::new("User", User::schema)
//! // }
//! ```

use crate::openapi::Schema;

/// Compile-time schema registration metadata
///
/// This struct holds metadata about a type that implements `ToSchema`.
/// Instances are collected at compile time via the `inventory` crate,
/// allowing the framework to discover all registered schemas without
/// explicit registration calls.
///
/// # Fields
///
/// * `name` - The schema name for $ref references (e.g., "User")
/// * `generator` - Function pointer to generate the schema
pub struct SchemaRegistration {
	/// Schema name for $ref references
	pub name: &'static str,
	/// Schema generator function
	pub generator: fn() -> Schema,
}

impl SchemaRegistration {
	/// Create a new schema registration entry
	///
	/// # Parameters
	///
	/// * `name` - The schema name (should match `ToSchema::schema_name()`)
	/// * `generator` - Function that generates the schema (typically `Type::schema`)
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use crate::openapi::{Schema, SchemaExt, SchemaRegistration, ToSchema};
	///
	/// struct User {
	///     id: i64,
	///     name: String,
	/// }
	///
	/// impl ToSchema for User {
	///     fn schema() -> Schema {
	///         Schema::object_with_properties(
	///             vec![
	///                 ("id", Schema::integer()),
	///                 ("name", Schema::string()),
	///             ],
	///             vec!["id", "name"],
	///         )
	///     }
	/// }
	///
	/// const REGISTRATION: SchemaRegistration = SchemaRegistration::new("User", User::schema);
	/// ```
	pub const fn new(name: &'static str, generator: fn() -> Schema) -> Self {
		Self { name, generator }
	}
}

// Enable inventory collection for SchemaRegistration
//
// This allows the `#[derive(Schema)]` macro to submit instances:
// ```
// inventory::submit! {
//     SchemaRegistration::new("TypeName", TypeName::schema)
// }
// ```
inventory::collect!(SchemaRegistration);
