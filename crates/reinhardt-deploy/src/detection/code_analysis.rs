use std::path::Path;

use regex::Regex;
use walkdir::WalkDir;

use crate::config::NoSqlEngine;
use crate::detection::feature_flags::FeatureDetectionResult;
use crate::error::{DeployError, DeployResult};

/// Analyze source code to detect infrastructure requirements.
///
/// Walks the `src/` directory recursively, scanning `.rs` files for
/// `use reinhardt::*` import patterns and `#[model(` attribute patterns.
/// Returns a [`FeatureDetectionResult`] with confidence 0.8 (code analysis
/// provides moderate confidence since imports may not reflect runtime usage).
///
/// If the `src/` directory does not exist, returns an empty result with
/// confidence 0.0.
pub fn analyze_code(project_root: &Path) -> DeployResult<FeatureDetectionResult> {
	let src_dir = project_root.join("src");

	if !src_dir.exists() {
		return Ok(FeatureDetectionResult {
			database: false,
			database_engine: None,
			nosql: false,
			nosql_engines: Vec::new(),
			cache: false,
			websockets: false,
			frontend: false,
			static_files: false,
			media: false,
			background_tasks: false,
			mail: false,
			wasm: false,
			ambiguous: false,
			model_count: 0,
			confidence: 0.0,
		});
	}

	let mut result = FeatureDetectionResult {
		database: false,
		database_engine: None,
		nosql: false,
		nosql_engines: Vec::new(),
		cache: false,
		websockets: false,
		frontend: false,
		static_files: false,
		media: false,
		background_tasks: false,
		mail: false,
		wasm: false,
		ambiguous: false,
		model_count: 0,
		confidence: 0.8,
	};

	// Patterns for `use reinhardt::*` imports
	let re_db = Regex::new(r"use\s+reinhardt::(db|orm)::").map_err(|e| DeployError::Detection {
		message: e.to_string(),
	})?;
	let re_websockets = Regex::new(r"use\s+reinhardt::(channels|websocket)::").map_err(|e| {
		DeployError::Detection {
			message: e.to_string(),
		}
	})?;
	let re_cache = Regex::new(r"use\s+reinhardt::cache::").map_err(|e| DeployError::Detection {
		message: e.to_string(),
	})?;
	let re_frontend = Regex::new(r"use\s+reinhardt::(pages|views::template)::?").map_err(|e| {
		DeployError::Detection {
			message: e.to_string(),
		}
	})?;
	let re_static_files =
		Regex::new(r"use\s+reinhardt::static_files::").map_err(|e| DeployError::Detection {
			message: e.to_string(),
		})?;
	let re_media = Regex::new(r"use\s+reinhardt::media::").map_err(|e| DeployError::Detection {
		message: e.to_string(),
	})?;
	let re_tasks =
		Regex::new(r"use\s+reinhardt::(tasks|celery)::").map_err(|e| DeployError::Detection {
			message: e.to_string(),
		})?;
	let re_mail = Regex::new(r"use\s+reinhardt::mail::").map_err(|e| DeployError::Detection {
		message: e.to_string(),
	})?;
	let re_nosql_mongodb =
		Regex::new(r"use\s+reinhardt::nosql::mongodb::").map_err(|e| DeployError::Detection {
			message: e.to_string(),
		})?;
	let re_nosql_dynamodb =
		Regex::new(r"use\s+reinhardt::nosql::dynamodb::").map_err(|e| DeployError::Detection {
			message: e.to_string(),
		})?;
	let re_nosql_firestore =
		Regex::new(r"use\s+reinhardt::nosql::firestore::").map_err(|e| DeployError::Detection {
			message: e.to_string(),
		})?;

	let re_wasm = Regex::new(r"use\s+reinhardt::(wasm|pages::wasm)::").map_err(|e| {
		DeployError::Detection {
			message: e.to_string(),
		}
	})?;

	// Pattern for `#[model(` attribute
	let re_model = Regex::new(r"#\[model\(").map_err(|e| DeployError::Detection {
		message: e.to_string(),
	})?;

	for entry in WalkDir::new(&src_dir).into_iter() {
		let entry = entry.map_err(|e| DeployError::Detection {
			message: format!("failed to walk directory: {e}"),
		})?;
		if !entry.file_type().is_file() || entry.path().extension().is_none_or(|ext| ext != "rs") {
			continue;
		}
		let content =
			std::fs::read_to_string(entry.path()).map_err(|e| DeployError::Detection {
				message: format!("failed to read {}: {}", entry.path().display(), e),
			})?;

		if re_db.is_match(&content) {
			result.database = true;
		}
		if re_websockets.is_match(&content) {
			result.websockets = true;
		}
		if re_cache.is_match(&content) {
			result.cache = true;
		}
		if re_frontend.is_match(&content) {
			result.frontend = true;
		}
		if re_static_files.is_match(&content) {
			result.static_files = true;
		}
		if re_media.is_match(&content) {
			result.media = true;
		}
		if re_tasks.is_match(&content) {
			result.background_tasks = true;
		}
		if re_mail.is_match(&content) {
			result.mail = true;
		}
		if re_wasm.is_match(&content) {
			result.wasm = true;
		}
		if re_nosql_mongodb.is_match(&content) {
			result.nosql = true;
			if !result.nosql_engines.contains(&NoSqlEngine::MongoDb) {
				result.nosql_engines.push(NoSqlEngine::MongoDb);
			}
		}
		if re_nosql_dynamodb.is_match(&content) {
			result.nosql = true;
			if !result.nosql_engines.contains(&NoSqlEngine::DynamoDb) {
				result.nosql_engines.push(NoSqlEngine::DynamoDb);
			}
		}
		if re_nosql_firestore.is_match(&content) {
			result.nosql = true;
			if !result.nosql_engines.contains(&NoSqlEngine::Firestore) {
				result.nosql_engines.push(NoSqlEngine::Firestore);
			}
		}

		let model_count = re_model.find_iter(&content).count();
		if model_count > 0 {
			result.database = true;
			result.model_count += model_count;
		}
	}

	Ok(result)
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn detect_database_from_use_statements() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			r#"
use reinhardt::db::models::Model;
use reinhardt::db::QuerySet;

fn main() {}
"#,
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.database);
	}

	#[rstest]
	fn detect_database_from_orm_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(src_dir.join("main.rs"), "use reinhardt::orm::QuerySet;\n").unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.database);
	}

	#[rstest]
	fn detect_websockets_from_use_statements() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			r#"
use reinhardt::channels::WebSocketConsumer;

fn main() {}
"#,
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.websockets);
	}

	#[rstest]
	fn detect_websockets_from_websocket_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			"use reinhardt::websocket::Connection;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.websockets);
	}

	#[rstest]
	fn detect_cache_from_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			"use reinhardt::cache::CacheBackend;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.cache);
	}

	#[rstest]
	fn detect_frontend_from_pages_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			"use reinhardt::pages::TemplateView;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.frontend);
	}

	#[rstest]
	fn detect_frontend_from_views_template_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			"use reinhardt::views::template::Render;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.frontend);
	}

	#[rstest]
	fn detect_static_files_from_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			"use reinhardt::static_files::StaticHandler;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.static_files);
	}

	#[rstest]
	fn detect_media_from_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			"use reinhardt::media::FileStorage;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.media);
	}

	#[rstest]
	fn detect_background_tasks_from_tasks_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			"use reinhardt::tasks::TaskRunner;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.background_tasks);
	}

	#[rstest]
	fn detect_background_tasks_from_celery_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(src_dir.join("main.rs"), "use reinhardt::celery::Worker;\n").unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.background_tasks);
	}

	#[rstest]
	fn detect_mail_from_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			"use reinhardt::mail::EmailBackend;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.mail);
	}

	#[rstest]
	fn detect_model_attribute() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("models.rs"),
			r#"
#[model(table_name = "users")]
pub struct User {
    pub id: i64,
    pub name: String,
}
"#,
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.database);
		assert!(result.model_count > 0);
	}

	#[rstest]
	fn detect_multiple_models() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("models.rs"),
			r#"
#[model(table_name = "users")]
pub struct User {
    pub id: i64,
    pub name: String,
}

#[model(table_name = "posts")]
pub struct Post {
    pub id: i64,
    pub title: String,
}

#[model(table_name = "comments")]
pub struct Comment {
    pub id: i64,
    pub body: String,
}
"#,
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.database);
		assert_eq!(result.model_count, 3);
	}

	#[rstest]
	fn detect_nosql_mongodb_from_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			"use reinhardt::nosql::mongodb::Collection;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.nosql);
		assert_eq!(result.nosql_engines, vec![NoSqlEngine::MongoDb]);
	}

	#[rstest]
	fn detect_nosql_dynamodb_from_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			"use reinhardt::nosql::dynamodb::Table;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.nosql);
		assert_eq!(result.nosql_engines, vec![NoSqlEngine::DynamoDb]);
	}

	#[rstest]
	fn detect_nosql_firestore_from_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			"use reinhardt::nosql::firestore::Document;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.nosql);
		assert_eq!(result.nosql_engines, vec![NoSqlEngine::Firestore]);
	}

	#[rstest]
	fn detect_wasm_from_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			"use reinhardt::wasm::WasmComponent;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.wasm);
	}

	#[rstest]
	fn detect_wasm_from_pages_wasm_use() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("main.rs"),
			"use reinhardt::pages::wasm::WasmApp;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.wasm);
	}

	#[rstest]
	fn no_src_dir_returns_empty_result() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		// No src/ directory created

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(!result.database);
		assert!(!result.nosql);
		assert!(!result.cache);
		assert!(!result.websockets);
		assert!(!result.frontend);
		assert!(!result.static_files);
		assert!(!result.media);
		assert!(!result.background_tasks);
		assert!(!result.mail);
		assert_eq!(result.model_count, 0);
		assert_eq!(result.confidence, 0.0);
	}

	#[rstest]
	fn empty_src_dir_returns_default_result() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(!result.database);
		assert!(!result.nosql);
		assert!(!result.cache);
		assert!(!result.websockets);
		assert_eq!(result.model_count, 0);
		assert_eq!(result.confidence, 0.8);
	}

	#[rstest]
	fn detect_features_across_multiple_files() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(src_dir.join("db.rs"), "use reinhardt::db::models::Model;\n").unwrap();
		std::fs::write(
			src_dir.join("ws.rs"),
			"use reinhardt::channels::Consumer;\n",
		)
		.unwrap();
		std::fs::write(src_dir.join("email.rs"), "use reinhardt::mail::Mailer;\n").unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.database);
		assert!(result.websockets);
		assert!(result.mail);
		assert_eq!(result.confidence, 0.8);
	}

	#[rstest]
	fn detect_features_in_nested_directories() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let nested_dir = tmp.path().join("src").join("app").join("handlers");
		std::fs::create_dir_all(&nested_dir).unwrap();
		std::fs::write(
			nested_dir.join("cache_handler.rs"),
			"use reinhardt::cache::RedisBackend;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(result.cache);
	}

	#[rstest]
	fn non_rs_files_are_ignored() {
		// Arrange
		let tmp = tempfile::tempdir().unwrap();
		let src_dir = tmp.path().join("src");
		std::fs::create_dir_all(&src_dir).unwrap();
		std::fs::write(
			src_dir.join("config.toml"),
			"use reinhardt::db::models::Model;\n",
		)
		.unwrap();
		std::fs::write(
			src_dir.join("readme.md"),
			"use reinhardt::cache::Backend;\n",
		)
		.unwrap();

		// Act
		let result = analyze_code(tmp.path()).unwrap();

		// Assert
		assert!(!result.database);
		assert!(!result.cache);
	}
}
