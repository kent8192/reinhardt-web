//! Regression tests for reinhardt-web#4447 (macro half):
//!
//! `#[field(default = ...)]` previously parsed the expression but never
//! pushed it into `FieldMetadata.params`, so the autodetector emitted
//! `ColumnDefinition.default = None` even for plain boolean / integer /
//! string defaults. That left the SQL runner with `ADD COLUMN ... NOT NULL`
//! without a DEFAULT clause — guaranteed to fail (silently, on SQLite empty
//! tables) the moment any existing row was present.
//!
//! These tests exercise the macro end-to-end via the global registry: if the
//! propagation works the registered `FieldMetadata.params` contains the
//! serialized SQL fragment, and `to_model_state()` carries it into the
//! `FieldState.params` that `ColumnDefinition::from_field_state` consumes.

use reinhardt_db::migrations::model_registry::global_registry;
use reinhardt_macros::model;
use rstest::*;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[model(
	app_label = "field_default_propagation_test",
	table_name = "field_default_propagation_test_user"
)]
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct DefaultsUser {
	#[field(primary_key = true)]
	pub id: i64,
	#[field(default = false)]
	pub is_superuser: bool,
	#[field(default = true)]
	pub is_active: bool,
	#[field(default = 0)]
	pub login_count: i64,
	#[field(default = "pending", max_length = 32)]
	pub status: String,
}

#[rstest]
fn field_default_propagates_to_metadata() {
	let registry = global_registry();
	let metadata = registry
		.get_model("field_default_propagation_test", "DefaultsUser")
		.expect("DefaultsUser must be registered by the #[model] macro");

	let is_superuser = metadata
		.fields
		.get("is_superuser")
		.expect("is_superuser field");
	assert_eq!(
		is_superuser.params.get("default").map(String::as_str),
		Some("false"),
		"#[field(default = false)] must serialize to the SQL fragment `false`"
	);

	let is_active = metadata.fields.get("is_active").expect("is_active field");
	assert_eq!(
		is_active.params.get("default").map(String::as_str),
		Some("true"),
		"#[field(default = true)] must serialize to the SQL fragment `true`"
	);

	let login_count = metadata
		.fields
		.get("login_count")
		.expect("login_count field");
	assert_eq!(
		login_count.params.get("default").map(String::as_str),
		Some("0"),
		"integer defaults survive as their base-10 representation"
	);

	let status = metadata.fields.get("status").expect("status field");
	assert_eq!(
		status.params.get("default").map(String::as_str),
		Some("'pending'"),
		"string defaults are SQL-quoted so they slot into `DEFAULT <frag>`"
	);
}

#[rstest]
fn field_default_reaches_column_definition_via_field_state() {
	use reinhardt_db::migrations::ColumnDefinition;

	let registry = global_registry();
	let metadata = registry
		.get_model("field_default_propagation_test", "DefaultsUser")
		.expect("DefaultsUser must be registered");
	let model_state = metadata.to_model_state();

	let field = model_state
		.get_field("is_superuser")
		.expect("is_superuser field reachable via ModelState");
	let col = ColumnDefinition::from_field_state("is_superuser", field);

	assert_eq!(
		col.default.as_deref(),
		Some("false"),
		"ColumnDefinition.default must be Some(_) so the runner can emit DEFAULT"
	);
}
