#![cfg(feature = "migrations")]

use reinhardt_db::migrations::{
	FilesystemRepository, FilesystemSource, Migration, MigrationRepository, MigrationSource,
	Operation, SqlAssetContext, ast_parser::extract_migration_metadata_strict, upgrade_source,
};
use rstest::{fixture, rstest};
use std::fs;
use tempfile::TempDir;

const FORWARD: &str = "-- guard 🦀\r\nDO $guard$ BEGIN PERFORM 1; END $guard$;\r\n";
const REVERSE: &str = "-- reverse\nSELECT 'undo; unchanged';\n";

#[fixture]
fn asset_tree() -> TempDir {
	let directory = TempDir::new_in("/tmp").expect("temporary migration tree");
	let app = directory.path().join("example");
	fs::create_dir_all(app.join("sql")).expect("SQL directory");
	fs::write(app.join("sql/guard.sql"), FORWARD).expect("forward asset");
	fs::write(app.join("sql/undo.sql"), REVERSE).expect("reverse asset");
	directory
}

fn source(sql: &str, reverse: &str) -> String {
	format!(
		r#"// reinhardt-migration-source: 1
fn migration() -> Migration {{
    Migration::new("ignored_name", "ignored_app")
        .add_operation(Operation::RunSQL {{ sql: {sql}, reverse_sql: {reverse} }})
        .add_dependency("base", "0001_initial")
        .add_replacement("example", "0000_legacy")
        .atomic(false)
        .initial(true)
        .state_only(true)
        .database_only(false)
}}
"#
	)
}

#[rstest]
#[case(".to_owned()")]
#[case(".to_string()")]
#[case(".into()")]
#[case("")]
#[case(".to_owned().into()")]
#[case(".into::<>()")]
#[tokio::test]
async fn loads_forward_reverse_and_complete_metadata(asset_tree: TempDir, #[case] suffix: &str) {
	// Arrange
	let sql = format!("include_str!(\"sql/guard.sql\"){suffix}");
	let reverse = format!("Some(include_str!(r#\"sql/undo.sql\"#,){suffix})");
	fs::write(
		asset_tree.path().join("example/0001_guard.rs"),
		source(&sql, &reverse),
	)
	.expect("migration source");
	let mut expected = Migration::new("0001_guard", "example");
	expected.operations.push(Operation::RunSQL {
		sql: FORWARD.into(),
		reverse_sql: Some(REVERSE.into()),
	});
	expected
		.dependencies
		.push(("base".into(), "0001_initial".into()));
	expected
		.replaces
		.push(("example".into(), "0000_legacy".into()));
	expected.atomic = false;
	expected.initial = Some(true);
	expected.state_only = true;

	// Act
	let loaded = FilesystemSource::new(asset_tree.path())
		.all_migrations()
		.await
		.expect("load included SQL");

	// Assert
	assert_eq!(loaded.len(), 1);
	assert_eq!(
		serde_json::to_value(&loaded[0]).unwrap(),
		serde_json::to_value(expected).unwrap()
	);
}

#[rstest]
#[case("include_str!(concat!(\"sql/\", \"guard.sql\"))")]
#[case("include_str!(env!(\"SQL_ASSET\"))")]
#[case("include_str!(ASSET_PATH)")]
#[case("include_str!(\"sql/guard.sql\", \"extra\")")]
#[case("include_str!()")]
#[case("std::include_str!(\"sql/guard.sql\")")]
#[case("other_macro!(\"sql/guard.sql\")")]
#[case("include_str!(\"sql/guard.sql\").replace(\"guard\", \"changed\")")]
#[case("include_str!(\"sql/guard.sql\").into::<String>()")]
#[case("include_str!(\"sql/guard.sql\").to_owned(1)")]
#[tokio::test]
async fn rejects_unsupported_include_grammar(asset_tree: TempDir, #[case] expression: &str) {
	// Arrange
	let path = asset_tree.path().join("example/0001_guard.rs");
	fs::write(&path, source(expression, "None")).unwrap();
	// Act
	let error = FilesystemSource::new(asset_tree.path())
		.all_migrations()
		.await
		.unwrap_err();
	// Assert
	let diagnostic = error.to_string();
	assert!(diagnostic.contains("0001_guard.rs"), "{diagnostic}");
	assert!(
		diagnostic.contains("operations[0].RunSQL.sql"),
		"{diagnostic}"
	);
}

#[rstest]
fn pathless_apis_require_explicit_coordinates(asset_tree: TempDir) {
	// Arrange
	let text = source("include_str!(\"sql/guard.sql\").to_owned()", "None");
	let ast = syn::parse_file(&text).unwrap();
	// Act
	let parse_error = extract_migration_metadata_strict(&ast, "example", "0001_guard").unwrap_err();
	let upgrade_error = upgrade_source(&text).unwrap_err();
	// Assert
	for error in [parse_error, upgrade_error] {
		assert!(
			error
				.to_string()
				.contains("explicit source path and migration root")
		);
	}
	assert_eq!(
		fs::read_to_string(asset_tree.path().join("example/sql/guard.sql")).unwrap(),
		FORWARD
	);
}

#[rstest]
fn one_context_caches_first_read_by_file_identity(asset_tree: TempDir) {
	// Arrange
	let path = asset_tree.path().join("example/0001_guard.rs");
	let text = source("include_str!(\"sql/guard.sql\").into()", "None");
	fs::write(&path, &text).unwrap();
	let alias = asset_tree.path().join("example/sql/alias.txt");
	fs::hard_link(asset_tree.path().join("example/sql/guard.sql"), &alias).unwrap();
	let mut context = SqlAssetContext::new(asset_tree.path()).unwrap();
	let first = context
		.extract_migration_metadata(
			&syn::parse_file(&text).unwrap(),
			&path,
			"example",
			"0001_guard",
		)
		.unwrap();
	fs::write(&alias, "SELECT 'changed';").unwrap();
	let aliased_text = text.replace("guard.sql", "alias.txt");
	// Act
	let second = context
		.extract_migration_metadata(
			&syn::parse_file(&aliased_text).unwrap(),
			&path,
			"example",
			"0001_guard",
		)
		.unwrap();
	let mut fresh = SqlAssetContext::new(asset_tree.path()).unwrap();
	let third = fresh
		.extract_migration_metadata(
			&syn::parse_file(&aliased_text).unwrap(),
			&path,
			"example",
			"0001_guard",
		)
		.unwrap();
	// Assert
	assert_eq!(first.operations, second.operations);
	assert_eq!(
		third.operations,
		vec![Operation::RunSQL {
			sql: "SELECT 'changed';".into(),
			reverse_sql: None
		}]
	);
}

#[cfg(unix)]
#[rstest]
fn loads_large_tree_under_low_file_descriptor_limit(asset_tree: TempDir) {
	// Arrange: isolate the process-wide descriptor limit from other tests.
	if std::env::var_os("REINHARDT_SQL_ASSET_FD_CHILD").is_none() {
		let output = std::process::Command::new("sh")
			.args(["-c", "ulimit -n 64 && exec \"$@\"", "sql-asset-fd-check"])
			.arg(std::env::current_exe().unwrap())
			.args([
				"--exact",
				"loads_large_tree_under_low_file_descriptor_limit",
				"--nocapture",
			])
			.env("REINHARDT_SQL_ASSET_FD_CHILD", "1")
			.output()
			.unwrap();
		assert!(
			output.status.success(),
			"descriptor-limited load failed:\n{}\n{}",
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		);
		return;
	}
	let count = 128;
	for index in 0..count {
		fs::write(
			asset_tree.path().join(format!("example/sql/{index}.sql")),
			format!("SELECT {index};"),
		)
		.unwrap();
		fs::write(
			asset_tree
				.path()
				.join(format!("example/{index:04}_guard.rs")),
			source(&format!("include_str!(\"sql/{index}.sql\").into()"), "None"),
		)
		.unwrap();
	}
	let runtime = tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build()
		.unwrap();

	// Act
	let loaded = runtime
		.block_on(FilesystemSource::new(asset_tree.path()).all_migrations())
		.expect("sources and assets must not accumulate open handles");

	// Assert
	assert_eq!(loaded.len(), count);
	for (index, migration) in loaded.iter().enumerate() {
		assert_eq!(
			migration.operations,
			vec![Operation::RunSQL {
				sql: format!("SELECT {index};"),
				reverse_sql: None,
			}]
		);
	}
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn compares_resolved_cfg_entrypoints(asset_tree: TempDir, #[case] different: bool) {
	// Arrange
	let included = source("(include_str!(\"sql/guard.sql\")).to_string()", "None");
	let literal = source(
		&format!("{:?}.into()", if different { "SELECT 2;" } else { FORWARD }),
		"None",
	);
	fs::write(
		asset_tree.path().join("example/0001_guard.rs"),
		format!("#[cfg(any())]\n{included}\n#[cfg(not(any()))]\n{literal}"),
	)
	.unwrap();
	// Act
	let result = FilesystemSource::new(asset_tree.path())
		.all_migrations()
		.await;
	// Assert
	if different {
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("different semantics")
		);
	} else {
		assert_eq!(
			result.unwrap()[0].operations,
			vec![Operation::RunSQL {
				sql: FORWARD.into(),
				reverse_sql: None
			}]
		);
	}
}

#[rstest]
#[case("missing.sql", "cannot resolve asset")]
#[case("sql/binary.txt", "cannot read UTF-8 asset")]
#[case("sql", "regular UTF-8 text file")]
#[tokio::test]
async fn reports_reverse_asset_failures(
	asset_tree: TempDir,
	#[case] include: &str,
	#[case] cause: &str,
) {
	// Arrange
	fs::write(
		asset_tree.path().join("example/sql/binary.txt"),
		[0xff, 0xfe],
	)
	.unwrap();
	let reverse = format!("Some(include_str!({include:?}).into())");
	fs::write(
		asset_tree.path().join("example/0001_guard.rs"),
		source("\"SELECT 1;\".into()", &reverse),
	)
	.unwrap();
	// Act
	let error = FilesystemSource::new(asset_tree.path())
		.all_migrations()
		.await
		.unwrap_err()
		.to_string();
	// Assert
	for part in [
		"0001_guard.rs",
		"operations[0].RunSQL.reverse_sql",
		include,
		cause,
	] {
		assert!(error.contains(part), "missing {part:?} in {error}");
	}
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn rejects_absolute_and_escaping_paths(asset_tree: TempDir, #[case] absolute: bool) {
	// Arrange
	let outside = TempDir::new_in("/tmp").unwrap();
	let target = outside.path().join("outside.sql");
	fs::write(&target, "SELECT 99;").unwrap();
	let expression = if absolute {
		format!("include_str!({:?}).into()", target.to_str().unwrap())
	} else {
		// Both temporary directories share a parent, so traversal stays relative.
		format!(
			"include_str!({:?}).into()",
			format!(
				"../../{}/outside.sql",
				outside.path().file_name().unwrap().to_str().unwrap()
			)
		)
	};
	fs::write(
		asset_tree.path().join("example/0001_guard.rs"),
		source(&expression, "None"),
	)
	.unwrap();
	// Act
	let error = FilesystemSource::new(asset_tree.path())
		.all_migrations()
		.await
		.unwrap_err()
		.to_string();
	// Assert
	assert!(
		error.contains(if absolute {
			"absolute include paths"
		} else {
			"escapes migration root"
		}),
		"{error}"
	);
}

#[rstest]
#[tokio::test]
async fn rejects_assets_aliasing_any_migration_source(asset_tree: TempDir) {
	// Arrange
	let other = asset_tree.path().join("example/0002_other.rs");
	fs::write(&other, source("\"SELECT 2;\".into()", "None")).unwrap();
	fs::hard_link(&other, asset_tree.path().join("example/sql/alias.txt")).unwrap();
	fs::write(
		asset_tree.path().join("example/0001_guard.rs"),
		source("include_str!(\"sql/alias.txt\").into()", "None"),
	)
	.unwrap();
	// Act
	let error = FilesystemSource::new(asset_tree.path())
		.all_migrations()
		.await
		.unwrap_err()
		.to_string();
	// Assert
	assert!(
		error.contains("both a migration source and an SQL asset"),
		"{error}"
	);
}

#[cfg(unix)]
#[rstest]
#[tokio::test]
async fn resolves_shared_internal_links_and_visible_source_parent(asset_tree: TempDir) {
	// Arrange
	use std::os::unix::fs::symlink;
	let shared = asset_tree.path().join("shared");
	fs::create_dir(&shared).unwrap();
	fs::write(shared.join("shared.txt"), FORWARD).unwrap();
	symlink(&shared, asset_tree.path().join("example/assets")).unwrap();
	let outside = TempDir::new_in("/tmp").unwrap();
	fs::write(
		outside.path().join("guard.sql"),
		"SELECT 'wrong source parent';",
	)
	.unwrap();
	let target = outside.path().join("original.rs");
	fs::write(
		&target,
		source(
			"include_str!(\"assets/shared.txt\").into()",
			"Some(include_str!(\"../shared/shared.txt\").into())",
		),
	)
	.unwrap();
	symlink(&target, asset_tree.path().join("example/0001_guard.rs")).unwrap();
	// Act
	let loaded = FilesystemSource::new(asset_tree.path())
		.all_migrations()
		.await
		.unwrap();
	// Assert
	assert_eq!(
		loaded[0].operations,
		vec![Operation::RunSQL {
			sql: FORWARD.into(),
			reverse_sql: Some(FORWARD.into())
		}]
	);
}

#[cfg(unix)]
#[rstest]
#[tokio::test]
async fn rejects_asset_symlinks_outside_the_root(asset_tree: TempDir) {
	// Arrange
	use std::os::unix::fs::symlink;
	let outside = TempDir::new_in("/tmp").unwrap();
	fs::write(outside.path().join("asset.sql"), "SELECT 1;").unwrap();
	symlink(
		outside.path().join("asset.sql"),
		asset_tree.path().join("example/sql/escape.sql"),
	)
	.unwrap();
	fs::write(
		asset_tree.path().join("example/0001_guard.rs"),
		source("include_str!(\"sql/escape.sql\").into()", "None"),
	)
	.unwrap();
	// Act
	let error = FilesystemSource::new(asset_tree.path())
		.all_migrations()
		.await
		.unwrap_err()
		.to_string();
	// Assert
	assert!(error.contains("escapes migration root"), "{error}");
}

#[rstest]
#[tokio::test]
async fn deployed_copy_and_generated_source_need_no_original_assets(asset_tree: TempDir) {
	// Arrange
	fs::write(
		asset_tree.path().join("example/0001_guard.rs"),
		source(
			"include_str!(\"sql/guard.sql\").into()",
			"Some(include_str!(\"sql/undo.sql\").into())",
		),
	)
	.unwrap();
	let deployed = TempDir::new_in("/tmp").unwrap();
	fs::create_dir_all(deployed.path().join("example/sql")).unwrap();
	for file in ["0001_guard.rs", "sql/guard.sql", "sql/undo.sql"] {
		fs::copy(
			asset_tree.path().join("example").join(file),
			deployed.path().join("example").join(file),
		)
		.unwrap();
	}
	let loaded = FilesystemSource::new(deployed.path())
		.all_migrations()
		.await
		.unwrap();
	let generated = TempDir::new_in("/tmp").unwrap();
	let mut repository = FilesystemRepository::new(generated.path());
	repository.save(&loaded[0]).await.unwrap();
	// Act
	drop(asset_tree);
	drop(deployed);
	let standalone = FilesystemSource::new(generated.path())
		.all_migrations()
		.await
		.unwrap();
	// Assert
	assert_eq!(
		serde_json::to_value(standalone).unwrap(),
		serde_json::to_value(loaded).unwrap()
	);
	assert!(
		!fs::read_to_string(generated.path().join("example/0001_guard.rs"))
			.unwrap()
			.contains("include_str!")
	);
}

#[rstest]
#[tokio::test]
async fn referenced_sql_does_not_emit_ignored_migration_warning(asset_tree: TempDir) {
	use std::io::Write;
	use std::sync::{Arc, Mutex};
	use tracing::instrument::WithSubscriber;
	#[derive(Clone)]
	struct Capture(Arc<Mutex<Vec<u8>>>);
	impl Write for Capture {
		fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
			self.0.lock().unwrap().extend_from_slice(bytes);
			Ok(bytes.len())
		}
		fn flush(&mut self) -> std::io::Result<()> {
			Ok(())
		}
	}
	// Arrange
	fs::write(
		asset_tree.path().join("example/0001_guard.rs"),
		source(
			"include_str!(\"sql/guard.sql\").into()",
			"Some(include_str!(\"sql/undo.sql\").into())",
		),
	)
	.unwrap();
	fs::write(
		asset_tree.path().join("example/sql/orphan.sql"),
		"SELECT 0;",
	)
	.unwrap();
	let output = Capture(Arc::new(Mutex::new(Vec::new())));
	let writer = output.clone();
	let subscriber = tracing_subscriber::fmt()
		.without_time()
		.with_ansi(false)
		.with_writer(move || writer.clone())
		.finish();
	// Act
	FilesystemSource::new(asset_tree.path())
		.all_migrations()
		.with_subscriber(subscriber)
		.await
		.unwrap();
	// Assert
	let logs = String::from_utf8(output.0.lock().unwrap().clone()).unwrap();
	assert_eq!(
		logs.matches("Found SQL migration file").count(),
		1,
		"{logs}"
	);
	assert!(logs.contains("orphan.sql"), "{logs}");
	assert!(
		!logs.contains("guard.sql") && !logs.contains("undo.sql"),
		"{logs}"
	);
}

#[rstest]
#[tokio::test]
async fn deterministic_failure_uses_first_source_and_operation(asset_tree: TempDir) {
	// Arrange
	for (name, sql) in [
		("0002_later", "missing_later.sql"),
		("0001_first", "missing_first.sql"),
	] {
		fs::write(
			asset_tree.path().join(format!("example/{name}.rs")),
			source(
				&format!("include_str!({sql:?}).into()"),
				"Some(include_str!(\"missing_reverse.sql\").into())",
			),
		)
		.unwrap();
	}
	let filesystem = FilesystemSource::new(asset_tree.path());
	// Act
	let first = filesystem.all_migrations().await.unwrap_err().to_string();
	let repeated = filesystem.all_migrations().await.unwrap_err().to_string();
	// Assert
	assert_eq!(first, repeated);
	assert!(
		first.contains("0001_first.rs") && first.contains("missing_first.sql"),
		"{first}"
	);
	assert!(!first.contains("missing_reverse.sql"), "{first}");
}

#[rstest]
#[tokio::test]
async fn legacy_layout_resolves_non_sql_text_extension(asset_tree: TempDir) {
	// Arrange
	let parent = asset_tree.path().join("example/migrations");
	fs::create_dir(&parent).unwrap();
	fs::write(parent.join("payload.rs"), FORWARD).unwrap();
	fs::write(
		parent.join("0001_guard.rs"),
		source("include_str!(\"payload.rs\").into()", "None"),
	)
	.unwrap();
	// Act
	let migrations = FilesystemSource::new(asset_tree.path())
		.all_migrations()
		.await
		.unwrap();
	// Assert
	assert_eq!(migrations.len(), 1);
	assert_eq!(migrations[0].app_label, "example");
	assert_eq!(migrations[0].name, "0001_guard");
	assert_eq!(
		migrations[0].operations,
		vec![Operation::RunSQL {
			sql: FORWARD.into(),
			reverse_sql: None
		}]
	);
}

#[rstest]
#[tokio::test]
async fn includes_in_non_sql_fields_remain_unsupported(asset_tree: TempDir) {
	// Arrange
	let text = r#"fn migration() -> Migration {
    Migration::new("0001_guard", "example").add_operation(Operation::DropTable {
        name: include_str!("sql/guard.sql").into(),
    })
}"#;
	fs::write(asset_tree.path().join("example/0001_guard.rs"), text).unwrap();
	// Act
	let error = FilesystemSource::new(asset_tree.path())
		.all_migrations()
		.await
		.unwrap_err()
		.to_string();
	// Assert
	assert!(error.contains("operations[0].DropTable.name"), "{error}");
}

#[cfg(unix)]
#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn unreadable_sql_fails_only_when_referenced(asset_tree: TempDir, #[case] referenced: bool) {
	use std::os::unix::fs::PermissionsExt;
	struct PermissionsGuard {
		path: std::path::PathBuf,
		original: fs::Permissions,
	}
	impl Drop for PermissionsGuard {
		fn drop(&mut self) {
			fs::set_permissions(&self.path, self.original.clone()).unwrap();
		}
	}
	// Arrange
	fs::write(
		asset_tree.path().join("example/0001_guard.rs"),
		source("include_str!(\"sql/guard.sql\").into()", "None"),
	)
	.unwrap();
	let path = asset_tree.path().join(if referenced {
		"example/sql/guard.sql"
	} else {
		"example/sql/orphan.sql"
	});
	if !referenced {
		fs::write(&path, "SELECT 'ignored';").unwrap();
	}
	let _permissions = PermissionsGuard {
		original: fs::metadata(&path).unwrap().permissions(),
		path: path.clone(),
	};
	fs::set_permissions(&path, fs::Permissions::from_mode(0o0)).unwrap();
	assert!(fs::read(&path).is_err(), "fixture must be unreadable");
	// Act
	let result = FilesystemSource::new(asset_tree.path())
		.all_migrations()
		.await;
	// Assert
	if referenced {
		let diagnostic = result.unwrap_err().to_string();
		for part in [
			"0001_guard.rs",
			"operations[0].RunSQL.sql",
			"sql/guard.sql",
			"cannot open asset",
		] {
			assert!(diagnostic.contains(part), "{diagnostic}");
		}
		return;
	}
	let migrations = result.unwrap();
	assert_eq!(migrations.len(), 1);
	assert_eq!(
		migrations[0].operations,
		vec![Operation::RunSQL {
			sql: FORWARD.into(),
			reverse_sql: None
		}]
	);
}

#[rstest]
#[tokio::test]
async fn reloads_changed_deployed_assets(asset_tree: TempDir) {
	// Arrange
	fs::write(
		asset_tree.path().join("example/0001_guard.rs"),
		source("include_str!(\"sql/guard.sql\").to_owned()", "None"),
	)
	.expect("migration source");
	let source = FilesystemSource::new(asset_tree.path());
	let original = source.all_migrations().await.expect("original load");
	let changed = "SELECT 'changed deployment';\n";
	fs::write(asset_tree.path().join("example/sql/guard.sql"), changed).expect("asset edit");

	// Act
	let reloaded = source.all_migrations().await.expect("reloaded assets");

	// Assert
	assert_eq!(
		original[0].operations,
		vec![Operation::RunSQL {
			sql: FORWARD.into(),
			reverse_sql: None
		}]
	);
	assert_eq!(
		reloaded[0].operations,
		vec![Operation::RunSQL {
			sql: changed.into(),
			reverse_sql: None
		}]
	);
}
