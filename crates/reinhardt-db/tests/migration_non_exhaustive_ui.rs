//! External-consumer compile coverage for the `#[non_exhaustive]` migration types.
//!
//! The trybuild pass case proves that downstream crates can build migration
//! state, introspection, and registry metadata through constructors and
//! builders, and can match the migration enums with an explicit-error wildcard
//! arm. The fail cases prove that exhaustive `Operation` matches and
//! `TableInfo` / `DatabaseSchema` struct expressions are rejected outside
//! `reinhardt-db`.

#[cfg(feature = "migrations")]
#[test]
fn migration_types_require_builders_and_wildcard_matches_downstream() {
	let tests = trybuild::TestCases::new();
	tests.pass("tests/ui/migration_non_exhaustive/pass/builders_and_wildcards.rs");
	tests.compile_fail("tests/ui/migration_non_exhaustive/fail/operation_exhaustive_match.rs");
	tests.compile_fail("tests/ui/migration_non_exhaustive/fail/table_info_struct_literal.rs");
	tests.compile_fail("tests/ui/migration_non_exhaustive/fail/database_schema_struct_update.rs");
}
