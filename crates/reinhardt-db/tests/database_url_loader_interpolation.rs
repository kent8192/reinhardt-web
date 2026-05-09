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

// `reinhardt_db::DatabaseConnection` resolves to the ORM-level wrapper;
// the loader entry point lives on the lower-level backends type.
use reinhardt_db::backends::connection::DatabaseConnection;
use rstest::rstest;
use serial_test::serial;
use std::env;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

/// Drop-based env-var cleanup. Removes named keys when the guard is
/// dropped, even on panic.
struct EnvGuard(Vec<&'static str>);

impl Drop for EnvGuard {
	fn drop(&mut self) {
		for key in &self.0 {
			// SAFETY: env mutation in tests is protected by #[serial(env)].
			unsafe { env::remove_var(key) };
		}
	}
}

/// Build a `<temp>/settings/{base.toml,<profile>.toml}` tree and return
/// the temp dir + the base path the loader should be pointed at.
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
	let _guard = EnvGuard(vec!["IT4247_DB_HOST", "DATABASE_URL", "REINHARDT_ENV"]);
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
	let _guard = EnvGuard(vec!["IT4247_DB_HOST_OPT", "DATABASE_URL", "REINHARDT_ENV"]);
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
