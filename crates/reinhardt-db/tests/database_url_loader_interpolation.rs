// End-to-end regression coverage for issue #4247.
//
// PR #4239 made `DatabaseConnection::get_database_url_from_env_or_settings`
// opt into `TomlFileSource::with_interpolation()`. The interpolation tests
// in `reinhardt-conf/tests/interpolation.rs` build a fresh `TomlFileSource`,
// so they will keep passing even if a future refactor accidentally drops
// the `.with_interpolation()` call from the real loader. These tests
// exercise the *real* entry point and assert the `${VAR}` / `${VAR:-default}`
// patterns expand end-to-end, so that regression fails loudly here.
//
// These tests mutate `std::env`, so they MUST run under `#[serial(env)]`.
// `EnvGuard` ensures cleanup even on panic.

#![cfg(feature = "settings")]
// This file pins disk-reading behavior of
// `DatabaseConnection::get_database_url_from_env_or_settings`. That entry
// point is now `#[deprecated]` in favor of
// `DatabaseConnection::database_url_from`, but its on-disk interpolation
// guarantees still need a regression test, so deprecation noise is
// suppressed at the module level here.
#![allow(deprecated)]

// `reinhardt_db::DatabaseConnection` resolves to the ORM-level wrapper;
// the loader entry point lives on the lower-level backends type.
use reinhardt_db::backends::connection::DatabaseConnection;
use rstest::rstest;
use serial_test::serial;
use std::env;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

/// Drop-based env-var cleanup. Captures each key's value on
/// construction and restores it (or removes it if previously unset) on
/// drop, so the guard never leaks state into ambient env vars that
/// existed before the test ran.
struct EnvGuard(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl EnvGuard {
	fn new(keys: Vec<&'static str>) -> Self {
		let captured = keys.into_iter().map(|k| (k, env::var_os(k))).collect();
		Self(captured)
	}
}

impl Drop for EnvGuard {
	fn drop(&mut self) {
		for (key, prev) in &self.0 {
			// SAFETY: env mutation in tests is protected by #[serial(env)].
			unsafe {
				match prev {
					Some(value) => env::set_var(key, value),
					None => env::remove_var(key),
				}
			}
		}
	}
}

/// Build a `<temp>/settings/{base.toml,<profile>.toml}` tree and return
/// the owned `TempDir`. Callers point the loader at `temp.path()` and
/// must keep the returned guard alive for the duration of the test.
fn write_settings_dir(profile: &str, base_toml: &str) -> TempDir {
	let temp = TempDir::new().expect("create temp dir");
	let settings_dir = temp.path().join("settings");
	std::fs::create_dir_all(&settings_dir).expect("create settings dir");

	write_file(&settings_dir.join("base.toml"), base_toml);
	// Empty per-profile file keeps the loader's profile source happy
	// without adding extra noise to the assertion.
	write_file(&settings_dir.join(format!("{profile}.toml")), "");
	temp
}

fn write_file(path: &Path, contents: &str) {
	let mut f = std::fs::File::create(path).expect("create file");
	f.write_all(contents.as_bytes()).expect("write file");
}

#[rstest]
#[serial(env)]
fn loader_expands_env_var_in_host() {
	// Arrange — DATABASE_URL would short-circuit the settings path, so
	// list it in the guard alongside the test-specific keys.
	let _guard = EnvGuard::new(vec!["IT4247_DB_HOST", "DATABASE_URL", "REINHARDT_ENV"]);
	// SAFETY: serial-protected.
	unsafe {
		env::remove_var("DATABASE_URL");
		env::set_var("REINHARDT_ENV", "local");
		env::set_var("IT4247_DB_HOST", "production-db.example.com");
	}
	let temp = write_settings_dir(
		"local",
		r#"
[database]
engine = "postgresql"
name = "appdb"
user = "app"
password = "secret"
host = "${IT4247_DB_HOST}"
port = 5432
"#,
	);

	// Act
	let url =
		DatabaseConnection::get_database_url_from_env_or_settings(Some(temp.path().to_path_buf()))
			.expect("loader returns a URL");

	// Assert — the expanded host must appear and the literal pattern
	// must not survive the loader.
	assert!(
		url.contains("production-db.example.com"),
		"expected expanded host in URL, got: {url}"
	);
	assert!(
		!url.contains("${"),
		"URL still contains literal interpolation pattern: {url}"
	);
}

#[rstest]
#[serial(env)]
fn loader_uses_inline_default_when_var_unset() {
	// Arrange — declare the var in the guard even though we never set it,
	// so an ambient value from a prior test cannot leak in and silence
	// the inline `:-fallback` branch.
	let _guard = EnvGuard::new(vec!["IT4247_DB_HOST_OPT", "DATABASE_URL", "REINHARDT_ENV"]);
	// SAFETY: serial-protected.
	unsafe {
		env::remove_var("DATABASE_URL");
		env::remove_var("IT4247_DB_HOST_OPT");
		env::set_var("REINHARDT_ENV", "local");
	}
	let temp = write_settings_dir(
		"local",
		r#"
[database]
engine = "postgresql"
name = "appdb"
user = "app"
password = "secret"
host = "${IT4247_DB_HOST_OPT:-fallback-host}"
port = 5432
"#,
	);

	// Act
	let url =
		DatabaseConnection::get_database_url_from_env_or_settings(Some(temp.path().to_path_buf()))
			.expect("loader returns a URL");

	// Assert — the inline fallback must apply because the var is unset.
	assert!(
		url.contains("fallback-host"),
		"expected fallback host in URL, got: {url}"
	);
	assert!(
		!url.contains("${"),
		"URL still contains literal interpolation pattern: {url}"
	);
}

// Coverage for the new `database_url_from` entry point: callers pass an
// already-built composed settings value (anything that implements
// `HasCoreSettings`) and `database_url_from` must return
// `core.databases.default.to_url()` without touching disk or the
// environment.
mod database_url_from_api {
	use rstest::rstest;
	use std::collections::HashMap;
	use std::path::PathBuf;

	use reinhardt_conf::settings::core_settings::CoreSettings;
	use reinhardt_conf::settings::database_config::DatabaseConfig;
	use reinhardt_conf::settings::fragment::HasSettings;
	use reinhardt_db::backends::connection::DatabaseConnection;

	// Minimal composed-settings test double that satisfies `HasCoreSettings`
	// via the generic `HasSettings<CoreSettings>` blanket impl. This keeps
	// the unit test independent of the `#[settings(...)]` macro.
	struct StubProjectSettings {
		core: CoreSettings,
	}

	impl HasSettings<CoreSettings> for StubProjectSettings {
		fn get_settings(&self) -> &CoreSettings {
			&self.core
		}
	}

	fn stub_settings(databases: HashMap<String, DatabaseConfig>) -> StubProjectSettings {
		StubProjectSettings {
			core: CoreSettings {
				base_dir: PathBuf::from("."),
				secret_key: "stub-secret-key-for-tests".to_string(),
				databases,
				..Default::default()
			},
		}
	}

	#[rstest]
	fn database_url_from_returns_default_databases_to_url() {
		// Arrange
		let mut dbs = HashMap::new();
		dbs.insert("default".to_string(), DatabaseConfig::sqlite("app.db"));
		let settings = stub_settings(dbs);

		// Act
		let url = DatabaseConnection::database_url_from(&settings, None)
			.expect("default databases entry should resolve to a URL");

		// Assert
		assert_eq!(url, "sqlite:app.db");
	}

	#[rstest]
	fn database_url_from_honors_env_override_first() {
		// Arrange
		let mut dbs = HashMap::new();
		dbs.insert("default".to_string(), DatabaseConfig::sqlite("ignored.db"));
		let settings = stub_settings(dbs);

		// Act
		let url = DatabaseConnection::database_url_from(&settings, Some("postgres://override/db"))
			.expect("override should short-circuit");

		// Assert — override returned verbatim, default entry is not consulted
		assert_eq!(url, "postgres://override/db");
	}

	#[rstest]
	fn database_url_from_errors_when_default_missing() {
		// Arrange — empty databases map (no "default" entry)
		let settings = stub_settings(HashMap::new());

		// Act
		let result = DatabaseConnection::database_url_from(&settings, None);

		// Assert
		assert!(
			result.is_err(),
			"missing default database entry must surface as an error, got: {:?}",
			result.ok(),
		);
	}
}
