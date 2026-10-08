//! Byte-vector models through the public database features in isolated consumers.

use rstest::{fixture, rstest};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[fixture]
fn consumer() -> TempDir {
	tempfile::Builder::new()
		.prefix("reinhardt-facade-binary-")
		.tempdir_in("/tmp")
		.expect("create isolated consumer directory")
}

#[rstest]
fn database_features_support_binary_models_without_direct_macro_dependencies(consumer: TempDir) {
	// Arrange: the facade is the consumer's only Reinhardt dependency.
	let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
		.ancestors()
		.nth(2)
		.expect("facade tests are inside tests/");
	let dependency_path = toml::Value::String(repository.to_string_lossy().into_owned());
	fs::create_dir_all(consumer.path().join("src")).expect("create consumer source directory");
	fs::write(
		consumer.path().join("src/main.rs"),
		r#"use reinhardt::db::migrations::{FieldType, SqlDialect};
use reinhardt::db::migrations::model_registry::global_registry;

#[reinhardt::model(app_label = "binary_consumer", table_name = "binary_records")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct BinaryRecord {
    #[field(primary_key = true)]
    id: i64,
    payload: Vec<u8>,
    #[field(null = true)]
    optional_payload: Option<Vec<u8>>,
}

fn main() {
    let metadata = global_registry()
        .get_model("binary_consumer", "BinaryRecord")
        .expect("the model macro must register binary metadata");
    for (name, nullable) in [("payload", false), ("optional_payload", true)] {
        let field = &metadata.fields[name];
        assert_eq!(field.field_type, FieldType::Binary);
        assert_eq!(field.nullable, nullable);
        for dialect in [SqlDialect::Postgres, SqlDialect::Cockroachdb] {
            assert_eq!(field.field_type.to_sql_for_dialect(&dialect), "BYTEA");
        }
    }
}
"#,
	)
	.expect("write consumer model");

	// Act: resolve each backend independently, reusing only this consumer's artifacts.
	for feature in ["db-postgres", "db-cockroachdb"] {
		fs::write(
			consumer.path().join("Cargo.toml"),
			format!(
				r#"[package]
name = "facade-binary-model-consumer"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
reinhardt = {{ package = "reinhardt-web", path = {dependency_path}, default-features = false, features = ["{feature}"] }}
serde = {{ version = "1", features = ["derive"] }}
ctor = "0.8"
"#,
			),
		)
		.expect("write facade-only consumer manifest");
		let output = Command::new(env!("CARGO"))
			.args(["run", "--quiet", "--offline", "--manifest-path"])
			.arg(consumer.path().join("Cargo.toml"))
			.current_dir(consumer.path())
			.env("CARGO_TARGET_DIR", consumer.path().join("target"))
			.env("CARGO_BUILD_BUILD_DIR", consumer.path().join("build"))
			.output()
			.expect("run isolated consumer");

		// Assert: compilation and runtime registration both work through public features.
		assert_eq!(
			output.status.code(),
			Some(0),
			"facade {feature} must support required and optional byte-vector models:\n{}",
			String::from_utf8_lossy(&output.stderr),
		);
	}
}
