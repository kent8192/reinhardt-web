//! Integration test for Schema derive macro dependency resolution.
//!
//! This test verifies that the Schema derive macro works correctly with only
//! the `openapi` feature enabled, without requiring:
//! 1. The `rest` feature to be explicitly enabled (should be automatic)
//! 2. The `inventory` crate as a direct dependency (should be re-exported)
//! 3. Manual import of the `ToSchema` trait (available via prelude)
//!
//! Related issue: Schema derive macro requires undocumented dependencies

use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};

/// Test struct with Schema derive macro
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct TestUser {
	pub id: i64,
	pub username: String,
	pub email: String,
	#[schema(example = "true")]
	pub is_active: bool,
}

/// Test enum with Schema derive macro
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub enum TestStatus {
	Active,
	Inactive,
	Pending,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_schema_derive_works_with_openapi_feature_only() {
		// Issue 1: rest feature should be enabled automatically by openapi feature
		// This test will compile only if the rest module is available

		// Issue 2: inventory crate should be available via re-export
		// The Schema derive macro uses inventory::submit! internally
		// This test will compile only if inventory is accessible

		// Issue 3: ToSchema trait should be available via prelude
		// We can call .schema() method which requires ToSchema to be in scope
		let schema = TestUser::schema();

		// Verify the schema is generated correctly
		assert!(matches!(schema, reinhardt::Schema::Object(_)));

		// Verify schema_name is correct
		assert_eq!(TestUser::schema_name(), Some("TestUser".to_string()));
	}

	#[test]
	fn test_enum_schema_derive_works() {
		// Test that enum schemas also work correctly
		let schema = TestStatus::schema();

		// Verify the schema is generated
		assert!(matches!(
			schema,
			reinhardt::Schema::String(_) | reinhardt::Schema::OneOf(_)
		));

		// Verify schema_name is correct
		assert_eq!(TestStatus::schema_name(), Some("TestStatus".to_string()));
	}

	#[test]
	fn test_schema_can_be_used_without_explicit_trait_import() {
		// This test verifies that using prelude::* makes ToSchema available
		// so users don't need to manually `use reinhardt::ToSchema;`

		// These calls to .schema() require ToSchema to be in scope
		let _user_schema = TestUser::schema();
		let _status_schema = TestStatus::schema();

		// If this compiles, it means the trait is available via prelude
	}
}
