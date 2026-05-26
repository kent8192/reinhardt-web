//! Shared state file for cross-macro communication between `installed_apps!`
//! and other macros.
//!
//! These proc macros expand within the same user crate, but cannot share data through
//! Rust's type system. This module provides file-based state sharing:
//!
//! - `installed_apps!` writes the list of app labels to a state file
//! - Other macros may read the file for app discovery
//!
//! This eliminates the need for the `#[macro_export]` callback pattern that triggers
//! `macro_expanded_macro_exports_accessed_by_absolute_paths` on Rust 1.94+.
//!
//! ## Why `CARGO_MANIFEST_DIR/target/` instead of `OUT_DIR`?
//!
//! `OUT_DIR` is only available during build-script execution, not during proc-macro
//! expansion. Proc macros have access to `CARGO_MANIFEST_DIR` via `std::env::var`,
//! so the state file is placed under `$CARGO_MANIFEST_DIR/target/reinhardt/`.
//!
//! ## Why a per-crate subdirectory? (Issue #4592)
//!
//! When a single Cargo manifest contains multiple compilation units that each expand
//! `installed_apps!` independently — most commonly the `[[test]]` targets under
//! `tests/integration/tests/*.rs` — cargo invokes rustc in parallel for every target.
//! All those rustc instances see the same `CARGO_MANIFEST_DIR`, so a flat state path
//! would race: one binary's `installed_apps!` would overwrite the labels another
//! binary's `#[routes]` is about to read, producing spurious E0433 errors.
//!
//! Cargo additionally sets `CARGO_CRATE_NAME` per compilation unit. We use it as a
//! subdirectory under `target/reinhardt/`, isolating each binary's state file.
//!
//! Both `CARGO_MANIFEST_DIR` and `CARGO_CRATE_NAME` are treated as hard requirements
//! and surface as `compile_error!` if missing. This matches the symmetric hard-fail
//! in the `include_bytes!` tracker emitted by `#[routes]`
//! (`routes_registration.rs`), which uses `env!("CARGO_MANIFEST_DIR")` and
//! `env!("CARGO_CRATE_NAME")` — `concat!()` cannot consume `option_env!()` with a
//! compile-time fallback, so emitting a runtime sentinel on one side and a
//! compile-time hard-fail on the other would be inconsistent. Both sides hard-fail
//! together (Issue #4592 / CodeRabbit thread).

use std::path::PathBuf;

/// File name for the installed apps state.
const STATE_FILE_NAME: &str = ".installed_apps";

/// Subdirectory under `target/` for reinhardt state files.
const STATE_SUBDIR: &str = "reinhardt";

/// Composes the state directory path. Pure function — extracted so it can be unit
/// tested without mutating process env vars.
fn compose_state_dir_path(manifest_dir: &str, crate_name: &str) -> PathBuf {
	PathBuf::from(manifest_dir)
		.join("target")
		.join(STATE_SUBDIR)
		.join(crate_name)
}

/// Returns the directory path for state files:
/// `$CARGO_MANIFEST_DIR/target/reinhardt/$CARGO_CRATE_NAME/`.
fn state_dir_path() -> Result<PathBuf, String> {
	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
		"CARGO_MANIFEST_DIR not set. Cannot locate installed apps state file".to_string()
	})?;
	let crate_name = std::env::var("CARGO_CRATE_NAME").map_err(|_| {
		"CARGO_CRATE_NAME not set. Cannot namespace installed apps state file (Issue #4592)"
			.to_string()
	})?;
	Ok(compose_state_dir_path(&manifest_dir, &crate_name))
}

/// Writes the installed app labels to the state file.
///
/// Creates the directory structure if it does not exist.
/// Labels are written as newline-separated UTF-8 text.
///
/// Returns an error if the directory cannot be created or the file cannot be written.
pub(crate) fn write_installed_apps(labels: &[String]) -> Result<(), String> {
	let dir = state_dir_path()?;
	std::fs::create_dir_all(&dir)
		.map_err(|e| format!("Cannot create state directory {}: {e}", dir.display()))?;
	let path = dir.join(STATE_FILE_NAME);
	let content = labels.join("\n");
	std::fs::write(&path, content).map_err(|e| format!("Cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
	use super::*;

	// Note: `state_dir_path()` and `write_installed_apps()`
	// rely on `CARGO_MANIFEST_DIR` / `CARGO_CRATE_NAME` set by cargo in the rustc
	// invocation environment. Cargo propagates `CARGO_MANIFEST_DIR` to test
	// runtimes but NOT `CARGO_CRATE_NAME`, so we cannot meaningfully exercise the
	// runtime wrappers from a unit test without mutating process env (unsafe in
	// Rust 2024). Instead we unit-test the pure path-composition helper, which
	// captures the Issue #4592 invariant. End-to-end behavior is covered by
	// `tests/integration/tests/*.rs` cleanly compiling from an empty
	// `target/reinhardt/`.

	#[test]
	fn compose_appends_crate_name_as_final_segment() {
		let path = compose_state_dir_path("/tmp/manifest", "widget_test");
		assert_eq!(
			path,
			PathBuf::from("/tmp/manifest")
				.join("target")
				.join("reinhardt")
				.join("widget_test"),
		);
	}

	#[test]
	fn compose_produces_distinct_paths_for_distinct_crate_names() {
		// Core Issue #4592 invariant: different test binaries must not collide
		// on the state file.
		let a = compose_state_dir_path("/tmp/manifest", "test_a");
		let b = compose_state_dir_path("/tmp/manifest", "test_b");
		assert_ne!(a, b);
	}
}
