//! Admin field inference under independently unified database features.

use rstest::{fixture, rstest};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[fixture]
fn consumer() -> TempDir {
	tempfile::Builder::new()
		.prefix("reinhardt-admin-pgvector-")
		.tempdir_in("/tmp")
		.expect("create isolated consumer directory")
}

#[rstest]
fn admin_infers_fields_with_independent_pgvector_features(consumer: TempDir) {
	// Arrange
	let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
		.ancestors()
		.nth(2)
		.expect("facade tests are inside tests/");
	let admin_path = toml::Value::String(
		repository
			.join("crates/reinhardt-admin")
			.to_string_lossy()
			.into_owned(),
	);
	let db_path = toml::Value::String(
		repository
			.join("crates/reinhardt-db")
			.to_string_lossy()
			.into_owned(),
	);
	fs::copy(
		repository.join("Cargo.lock"),
		consumer.path().join("Cargo.lock"),
	)
	.expect("seed consumer dependency versions from workspace lockfile");
	fs::create_dir(consumer.path().join("src")).expect("create consumer source directory");
	let cases = [
		("database-only-vector", "", "pgvector", true),
		("without-vector", "", "migrations", false),
		("admin-vector", "pgvector", "pgvector", true),
	];

	// Act: replace each manifest while reusing only build artifacts. The consumer
	// graph cannot inherit reinhardt-admin/pgvector from a workspace test build.
	for (name, admin_feature, db_feature, vector_enabled) in cases {
		fs::write(
			consumer.path().join("Cargo.toml"),
			format!(
				r#"[package]
name = "admin-pgvector-consumer"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
reinhardt-admin = {{ path = {admin_path}, default-features = false, features = [{admin_features}] }}
reinhardt-db = {{ path = {db_path}, default-features = false, features = ["{db_feature}"] }}
"#,
				admin_features = if admin_feature.is_empty() {
					String::new()
				} else {
					toml::Value::String(admin_feature.into()).to_string()
				},
			),
		)
		.expect("write standalone consumer manifest");
		let vector_assertion = if vector_enabled {
			r#"
    let field = DbFieldType::Vector { dimensions: 1536 };
    assert_eq!(infer_admin_field_type(&field), AdminFieldType::TextArea);
"#
		} else {
			""
		};
		fs::write(
			consumer.path().join("src/main.rs"),
			format!(
				r#"use reinhardt_admin::server::type_inference::infer_admin_field_type;
use reinhardt_admin::types::FieldType as AdminFieldType;
use reinhardt_db::migrations::FieldType as DbFieldType;

fn main() {{
    assert_eq!(infer_admin_field_type(&DbFieldType::Integer), AdminFieldType::Number);
    assert_eq!(infer_admin_field_type(&DbFieldType::Custom("geometry".into())), AdminFieldType::Text);
{vector_assertion}}}
"#
			),
		)
		.expect("write consumer field inference assertions");
		let output = Command::new(env!("CARGO"))
			.args(["run", "--quiet", "--offline"])
			.current_dir(consumer.path())
			.env("CARGO_PROFILE_DEV_DEBUG", "0")
			.env("CARGO_INCREMENTAL", "0")
			.env("CARGO_TARGET_DIR", consumer.path().join("target"))
			.env("CARGO_BUILD_BUILD_DIR", consumer.path().join("build"))
			.output()
			.expect("run isolated consumer Cargo command");

		// Assert
		assert_eq!(
			output.status.code(),
			Some(0),
			"admin field inference must succeed for {name}:\n{}",
			String::from_utf8_lossy(&output.stderr),
		);
	}
}
