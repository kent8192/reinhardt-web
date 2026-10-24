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
	let mut workspace: toml::Value = toml::from_str(
		&fs::read_to_string(repository.join("Cargo.toml")).expect("read workspace manifest"),
	)
	.expect("parse workspace manifest");
	let patches = workspace["patch"]
		.as_table_mut()
		.expect("workspace patches");
	for (_, sources) in patches.iter_mut() {
		for (_, dependency) in sources
			.as_table_mut()
			.expect("patch source table")
			.iter_mut()
		{
			if let Some(path) = dependency.get_mut("path") {
				*path = toml::Value::String(
					repository
						.join(path.as_str().expect("patch path"))
						.to_string_lossy()
						.into_owned(),
				);
			}
		}
	}
	let patches = toml::to_string(&toml::Value::Table(toml::map::Map::from_iter([(
		"patch".into(),
		toml::Value::Table(patches.clone()),
	)])))
	.expect("serialize workspace patches for the isolated consumer");
	fs::copy(
		repository.join("Cargo.lock"),
		consumer.path().join("Cargo.lock"),
	)
	.expect("seed consumer dependency versions from workspace lockfile");
	fs::create_dir(consumer.path().join("src")).expect("create consumer source directory");
	let cases = [
		("database-only-vector", "", "pgvector", Some("Hidden")),
		("without-vector", "", "migrations", None),
		("admin-vector", "pgvector", "pgvector", Some("TextArea")),
	];

	// Act: replace each manifest while reusing only build artifacts. The consumer
	// graph cannot inherit reinhardt-admin/pgvector from a workspace test build.
	for (name, admin_feature, db_feature, vector_type) in cases {
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
serde_json = "1"

{patches}
"#,
				admin_features = if admin_feature.is_empty() {
					String::new()
				} else {
					toml::Value::String(admin_feature.into()).to_string()
				},
			),
		)
		.expect("write standalone consumer manifest");
		let vector_assertion = if let Some(vector_type) = vector_type {
			format!(
				r#"
    let field = DbFieldType::Vector {{ dimensions: 1536 }};
    assert_eq!(infer_admin_field_type(&field), AdminFieldType::{vector_type});
    struct RegisteredModel;
    impl Drop for RegisteredModel {{
        fn drop(&mut self) {{
            reinhardt_db::migrations::global_registry().remove_model("consumer", "Embedding");
        }}
    }}
    let mut model = reinhardt_db::migrations::ModelMetadata::new("consumer", "Embedding", "embeddings");
    model.add_field("embedding".into(), reinhardt_db::migrations::FieldMetadata::new(field));
    reinhardt_db::migrations::global_registry().register_model(model);
    let _registered = RegisteredModel;
    let admin = reinhardt_admin::core::ModelAdminConfig::builder()
        .model_name("Embedding").table_name("embeddings")
        .fields(vec!["embedding"]).build().unwrap();
    for value in [serde_json::json!([1.0, 2.0, 3.0]), serde_json::json!("[1,2,3]"), serde_json::Value::Null] {{
        let data = std::collections::HashMap::from([("embedding".into(), value)]);
        for is_update in [false, true] {{
            let result = reinhardt_admin::server::validation::validate_mutation_data(&data, &admin, is_update);
            assert_eq!(result.is_ok(), {editable});
            if let Err(error) = result {{
                assert_eq!(error.to_string(), "Validation error: Field 'embedding' requires an enabled admin feature before it can be edited");
            }}
        }}
    }}
"#,
				editable = vector_type == "TextArea",
			)
		} else {
			String::new()
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
"#,
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
