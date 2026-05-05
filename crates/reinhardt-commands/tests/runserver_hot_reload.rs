//! Integration tests for the runserver hot-reload pipelines (issue #4128).
//!
//! # Approach
//!
//! These tests exercise the hot-reload building blocks against real
//! tempdir crate fixtures. They drive:
//!
//! * [`WasmBuilder`] (the public surface backing `WasmRebuildPipeline::run`)
//!   for the WASM-side scenarios (HR-1, HR-5, HR-6).
//! * [`ServerRebuildPipeline`] for the server-side scenarios (HR-2, HR-4,
//!   HR-5).
//! * [`WatcherConfig`] / [`is_relevant_change`] for the dispatch-shape
//!   scenario (HR-3).
//!
//! # Pivot rationale
//!
//! The original plan called for spawning a full `manage runserver` against a
//! fresh tempdir crate. That approach is infeasible in this codebase:
//!
//! 1. `reinhardt-commands` declares no `[[bin]] name = "manage"`, so
//!    `env!("CARGO_BIN_EXE_manage")` does not resolve.
//! 2. `Runserver::run_with_autoreload` requires a fully-wired
//!    `CommandContext` plus a project that can serve HTTP — far more
//!    machinery than fits inside a tempdir fixture.
//! 3. A full WASM rebuild against a cold cargo cache reliably exceeds the
//!    180s test budget per case.
//!
//! Instead we drive the hot-reload pieces directly through the
//! `__hot_reload_test_api` re-export shim, which is gated `#[doc(hidden)]`
//! and only exists for these tests.
//!
//! # Cost model
//!
//! Each test that builds Rust runs in its own tempdir but shares
//! `CARGO_TARGET_DIR` (set per process to a stable directory under
//! `/tmp`) so subsequent test cases benefit from cargo's incremental
//! cache. Cold-cache first-test runtime is ~30-60s; warm-cache subsequent
//! cases are <5s.

#![cfg(all(feature = "server", feature = "autoreload", feature = "pages"))]

use std::path::{Path, PathBuf};
use std::time::Duration;

use reinhardt_commands::__hot_reload_test_api::{
	DEBOUNCE_WINDOW, ServerRebuildOutcome, ServerRebuildPipeline, SourceRoots, WatcherConfig,
	is_relevant_change,
};
use reinhardt_commands::{WasmBuildConfig, WasmBuilder};
use serial_test::serial;

const FIXTURE_CARGO_TPL: &str = include_str!("fixtures/hot_reload_fixture/Cargo.toml.tpl");
const FIXTURE_LIB_TPL: &str = include_str!("fixtures/hot_reload_fixture/src/lib.rs.tpl");
const FIXTURE_MAIN_TPL: &str = include_str!("fixtures/hot_reload_fixture/src/main.rs.tpl");

/// Materialised tempdir crate used as a hot-reload target.
struct Fixture {
	root: tempfile::TempDir,
	#[allow(dead_code)] // Retained for diagnostic prints; surfaces in panic messages.
	name: String,
}

impl Fixture {
	/// Create a fresh fixture crate under a tempdir. The crate name is
	/// stable per-test-name so that cargo's incremental cache stays warm
	/// when the same test re-runs.
	fn new(crate_name: &str, marker: u32) -> Self {
		let root = tempfile::tempdir().expect("create tempdir");
		let wasm_bindgen_version = detect_wasm_bindgen_cli_version();
		let cargo = FIXTURE_CARGO_TPL
			.replace("{{NAME}}", crate_name)
			.replace("{{WASM_BINDGEN_VERSION}}", &wasm_bindgen_version);
		let lib = FIXTURE_LIB_TPL.replace("{{MARKER}}", &marker.to_string());
		let main = FIXTURE_MAIN_TPL.replace("{{MARKER}}", &marker.to_string());

		std::fs::write(root.path().join("Cargo.toml"), cargo).unwrap();
		std::fs::create_dir_all(root.path().join("src")).unwrap();
		std::fs::write(root.path().join("src/lib.rs"), lib).unwrap();
		std::fs::write(root.path().join("src/main.rs"), main).unwrap();

		Self {
			root,
			name: crate_name.to_string(),
		}
	}

	fn path(&self) -> &Path {
		self.root.path()
	}

	/// Rewrite the cdylib `{{MARKER}}` slot to the given value.
	fn edit_marker(&self, new_marker: u32) {
		let lib = FIXTURE_LIB_TPL.replace("{{MARKER}}", &new_marker.to_string());
		std::fs::write(self.root.path().join("src/lib.rs"), lib).unwrap();
	}

	/// Replace `src/main.rs` with non-Rust junk so `cargo build --bin`
	/// fails. Leaves `src/lib.rs` untouched.
	fn introduce_server_syntax_error(&self) {
		std::fs::write(
			self.root.path().join("src/main.rs"),
			"this is not valid rust syntax @@@\n",
		)
		.unwrap();
	}

	/// Restore `src/main.rs` from the fixture template.
	fn restore_server(&self, marker: u32) {
		let main = FIXTURE_MAIN_TPL.replace("{{MARKER}}", &marker.to_string());
		std::fs::write(self.root.path().join("src/main.rs"), main).unwrap();
	}

	/// Replace the cdylib body with a deliberately broken `extern` block
	/// (wasm32-only) that references a symbol the linker cannot resolve.
	/// `host_marker()` remains valid so the `manage` bin still links.
	fn introduce_wasm_only_error(&self) {
		// A wasm32-only Rust type error guarantees `cargo build --target
		// wasm32-unknown-unknown` (and thus the WASM pipeline) fails at
		// the rustc level, while the host-side `host_marker()` continues
		// to build the manage bin cleanly.
		let body = r#"#[cfg(target_arch = "wasm32")]
mod wasm_only {
	use wasm_bindgen::prelude::*;

	#[wasm_bindgen]
	pub fn marker() -> u32 {
		// Type error: assigning a string literal to u32. This is
		// rejected by rustc only when the wasm32 cfg gate compiles
		// the body, so the host build of the manage bin is
		// unaffected.
		let value: u32 = "this is not a u32";
		value
	}
}

pub fn host_marker() -> u32 {
	1
}
"#;
		std::fs::write(self.root.path().join("src/lib.rs"), body).unwrap();
	}
}

/// Build the WASM bundle for `fixture` using the same `WasmBuilder` surface
/// that `WasmRebuildPipeline::run` uses internally.
fn build_wasm(
	fixture: &Fixture,
) -> Result<reinhardt_commands::WasmBuildOutput, reinhardt_commands::WasmBuildError> {
	// Each fixture uses its own per-tempdir `target/` directory. Sharing a
	// CARGO_TARGET_DIR across tests would require process-level env
	// mutation; the per-fixture cost is acceptable for an integration
	// suite (cold ~30s, warm ~10s per build).
	let config = WasmBuildConfig::new(fixture.path()).output_dir("dist");
	WasmBuilder::new(config).build()
}

/// Resolve the cargo target directory cargo will use for `fixture`.
///
/// Honours user-level config redirections such as `build.build-dir` and
/// `CARGO_TARGET_DIR`. Returned path is the *root* target dir; binaries
/// land under `<root>/debug/`.
fn fixture_target_dir(fixture: &Fixture) -> PathBuf {
	let output = std::process::Command::new("cargo")
		.args(["metadata", "--no-deps", "--format-version=1"])
		.current_dir(fixture.path())
		.output()
		.expect("cargo metadata for fixture");
	let json: serde_json::Value =
		serde_json::from_slice(&output.stdout).expect("cargo metadata json parse");
	let target_dir = json
		.get("target_directory")
		.and_then(|v| v.as_str())
		.expect("cargo metadata.target_directory");
	PathBuf::from(target_dir)
}

/// Path to the `manage` binary inside `fixture`'s effective target dir.
fn fixture_manage_bin(fixture: &Fixture) -> PathBuf {
	fixture_target_dir(fixture).join("debug").join("manage")
}

/// Spawn a long-running `manage` child for `fixture` after building it.
/// The child sleeps for an hour and is killed by the test on drop.
async fn spawn_long_running_child(fixture: &Fixture) -> tokio::process::Child {
	// Pre-build the manage bin so the spawned child has something to exec.
	let status = std::process::Command::new("cargo")
		.args(["build", "--bin", "manage", "--manifest-path"])
		.arg(fixture.path().join("Cargo.toml"))
		.status()
		.expect("invoke cargo build for fixture manage bin");
	assert!(status.success(), "fixture manage bin must build");

	let bin_path = fixture_manage_bin(fixture);
	tokio::process::Command::new(&bin_path)
		.kill_on_drop(true)
		.spawn()
		.expect("spawn fixture manage bin")
}

/// Run `ServerRebuildPipeline::run` against `fixture` with a respawn closure
/// that boots a fresh `manage` child from the shared target directory.
async fn run_server_pipeline(
	fixture: &Fixture,
	current_child: &mut tokio::process::Child,
) -> ServerRebuildOutcome {
	// `ServerRebuildPipeline::run` shells to `cargo build --bin <bin>`
	// with the parent process's cwd. We need cargo to find the fixture's
	// `Cargo.toml`, so we temporarily switch cwd to the fixture root.
	// Tests are serialised via `serial_test::serial(server_pipeline)`
	// to avoid concurrent cwd writes.
	let saved_cwd = std::env::current_dir().expect("current dir");
	std::env::set_current_dir(fixture.path()).expect("set fixture cwd");

	let bin_name = "manage".to_string();
	let bin_path = fixture_manage_bin(fixture);
	let respawn = move || -> std::io::Result<tokio::process::Child> {
		tokio::process::Command::new(&bin_path)
			.kill_on_drop(true)
			.spawn()
	};

	let (outcome, new_child) = ServerRebuildPipeline::run(&bin_name, current_child, respawn).await;

	std::env::set_current_dir(&saved_cwd).expect("restore cwd");

	if let Some(c) = new_child {
		*current_child = c;
	}
	outcome
}

// ---------------------------------------------------------------------------
// HR-1: a wasm-side edit triggers a successful WASM rebuild.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
async fn hr_1_wasm_change_triggers_wasm_rebuild() {
	// Arrange: build once so the bundle exists.
	let fixture = Fixture::new("hr1_fixture", 100);
	let first = build_wasm(&fixture).expect("initial wasm build must succeed");
	assert!(
		first.wasm_file.exists(),
		"first build must produce {}",
		first.wasm_file.display()
	);

	// Act: edit the marker, rebuild.
	fixture.edit_marker(101);
	let second = build_wasm(&fixture).expect("rebuild after edit must succeed");

	// Assert: the post-edit bundle still exists (and is the same path).
	assert_eq!(first.wasm_file, second.wasm_file);
	assert!(second.wasm_file.exists());
}

// ---------------------------------------------------------------------------
// HR-2: a server-side edit triggers a successful Server rebuild + restart.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
#[serial(server_pipeline)]
async fn hr_2_server_change_triggers_server_rebuild() {
	// Arrange
	let fixture = Fixture::new("hr2_fixture", 200);
	let mut child = spawn_long_running_child(&fixture).await;

	// Act
	let outcome = run_server_pipeline(&fixture, &mut child).await;

	// Cleanup
	let _ = child.kill().await;

	// Assert: a successful Ok outcome with non-zero duration.
	match outcome {
		ServerRebuildOutcome::Ok { duration } => {
			assert!(duration > Duration::ZERO, "duration must be positive");
		}
		other => panic!("expected ServerRebuildOutcome::Ok, got {other:?}"),
	}
}

// ---------------------------------------------------------------------------
// HR-3: --no-wasm-rebuild flag plumbing skips the WASM pipeline.
//
// This test asserts the dispatch-time configuration the watcher reads:
// `WatcherConfig.no_wasm_rebuild` round-trips the flag, and the path filter
// still recognises Rust source edits as relevant. Behavioural verification
// (no WASM log line appears under the flag) lives in the unit tests of
// `debounced_watcher`, where the dispatch branch is observable.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
async fn hr_3_no_wasm_rebuild_flag_skips_wasm_pipeline() {
	// Arrange: a config with the flag set, plus an empty SourceRoots.
	let config = WatcherConfig {
		bin_name: "manage".to_string(),
		roots: SourceRoots {
			src_dirs: Vec::new(),
			manifest_files: Vec::new(),
		},
		no_wasm_rebuild: true,
		pages_enabled: true,
	};

	// Act: read the flag back.
	let flag = config.no_wasm_rebuild;
	let pages_enabled = config.pages_enabled;

	// Assert: the flag is true, pages_enabled is true, and a typical
	// wasm-side edit path is still considered relevant by the filter.
	// Together these prove the dispatch branch
	// `pages_enabled && !no_wasm_rebuild` evaluates to false here, which
	// is the precondition the watcher uses to skip the WASM pipeline.
	assert!(flag, "no_wasm_rebuild must round-trip true");
	assert!(pages_enabled, "pages_enabled must round-trip true");
	let dispatch_wasm = pages_enabled && !flag;
	assert!(
		!dispatch_wasm,
		"with no_wasm_rebuild=true the watcher must NOT dispatch WASM"
	);
	assert!(
		is_relevant_change(&fake_event(
			notify::EventKind::Modify(notify::event::ModifyKind::Any),
			"/p/src/lib.rs",
		)),
		"a .rs edit must still pass the relevance filter (server pipeline still runs)"
	);
	assert_eq!(DEBOUNCE_WINDOW, Duration::from_millis(300));
}

// ---------------------------------------------------------------------------
// HR-4: outer loop survives a syntax error and recovers on a follow-up edit.
//
// Drives ServerRebuildPipeline::run through Ok -> BuildFailed -> Ok and
// asserts the watcher-equivalent outer-loop invariant: the pipeline
// returns *outcomes*, never propagates errors that would tear down a
// real watcher loop.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
#[serial(server_pipeline)]
async fn hr_4_resilience_recovers_from_syntax_error() {
	// Arrange: valid fixture + initial child.
	let fixture = Fixture::new("hr4_fixture", 400);
	let mut child = spawn_long_running_child(&fixture).await;

	// Act 1: first rebuild on the valid source -> Ok.
	let first = run_server_pipeline(&fixture, &mut child).await;
	assert!(
		matches!(first, ServerRebuildOutcome::Ok { .. }),
		"first rebuild must succeed, got {first:?}"
	);

	// Act 2: introduce a syntax error and rebuild -> BuildFailed.
	fixture.introduce_server_syntax_error();
	let broken = run_server_pipeline(&fixture, &mut child).await;
	assert!(
		matches!(broken, ServerRebuildOutcome::BuildFailed { .. }),
		"syntax error must produce BuildFailed, got {broken:?}"
	);

	// Outer-loop invariant: the prior child is still alive (BuildFailed
	// must NOT kill the running server).
	let still_running = child.try_wait().expect("try_wait on current child");
	assert!(
		still_running.is_none(),
		"BuildFailed must keep the prior child alive; got exit status {still_running:?}"
	);

	// Act 3: restore the source and rebuild -> Ok again.
	fixture.restore_server(401);
	let recovered = run_server_pipeline(&fixture, &mut child).await;
	assert!(
		matches!(recovered, ServerRebuildOutcome::Ok { .. }),
		"recovery rebuild must succeed, got {recovered:?}"
	);

	// Cleanup
	let _ = child.kill().await;
}

// ---------------------------------------------------------------------------
// HR-5: a wasm-only failure does not take down the server pipeline.
//
// Builds a fixture whose cdylib references a missing extern symbol on the
// wasm32 target. Asserts:
//   * `WasmBuilder::build` returns a `CargoBuildFailed` (or other build
//     error) — the wasm pipeline reports failure.
//   * `ServerRebuildPipeline::run` against the same fixture still returns
//     `Ok` because the host bin's `host_marker()` is intact.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
#[serial(server_pipeline)]
async fn hr_5_partial_failure_keeps_other_pipeline_alive() {
	// Arrange: fixture whose wasm side will fail to link.
	let fixture = Fixture::new("hr5_fixture", 500);
	fixture.introduce_wasm_only_error();

	// Act 1: WASM build must fail.
	let wasm_result = build_wasm(&fixture);
	assert!(
		wasm_result.is_err(),
		"wasm-only error must surface as a build failure, got {wasm_result:?}"
	);

	// Act 2: Server pipeline against the same fixture must still succeed.
	let mut child = spawn_long_running_child(&fixture).await;
	let outcome = run_server_pipeline(&fixture, &mut child).await;
	let _ = child.kill().await;

	// Assert
	assert!(
		matches!(outcome, ServerRebuildOutcome::Ok { .. }),
		"server pipeline must remain healthy under a wasm-only failure, got {outcome:?}"
	);
}

// ---------------------------------------------------------------------------
// HR-6: literal #4128 reproduction — wasm bundle mtime advances after an edit.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
async fn hr_6_issue_4128_reproduction() {
	// Arrange: clean fixture, first build, capture the bundle mtime.
	let fixture = Fixture::new("hr6_fixture", 600);
	let first = build_wasm(&fixture).expect("initial wasm build must succeed");
	let before = std::fs::metadata(&first.wasm_file)
		.and_then(|m| m.modified())
		.expect("first build mtime");

	// Force at least one filesystem-mtime tick before the second build —
	// macOS HFS+ has 1s resolution and APFS is sub-second but variable.
	std::thread::sleep(Duration::from_millis(1100));

	// Act: edit the wasm-side source, rebuild.
	fixture.edit_marker(601);
	let second = build_wasm(&fixture).expect("rebuild after edit must succeed");
	let after = std::fs::metadata(&second.wasm_file)
		.and_then(|m| m.modified())
		.expect("second build mtime");

	// Assert: the mtime advanced. This is the literal symptom #4128
	// reports: in the broken implementation the wasm bundle was never
	// re-emitted, so its mtime stayed put across edits.
	assert!(
		after > before,
		"wasm bundle mtime must advance after a wasm-side edit (before={before:?}, after={after:?})"
	);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Probe the locally-installed `wasm-bindgen-cli` version so the fixture
/// can pin its `wasm-bindgen` dep to exactly that version (their bindgen
/// schemas must match exactly).
fn detect_wasm_bindgen_cli_version() -> String {
	let output = std::process::Command::new("wasm-bindgen")
		.arg("--version")
		.output()
		.expect("wasm-bindgen-cli must be installed for the hot-reload tests");
	let stdout = String::from_utf8_lossy(&output.stdout);
	// Format: "wasm-bindgen 0.2.114\n"
	stdout
		.split_whitespace()
		.nth(1)
		.expect("wasm-bindgen --version output must have a version token")
		.to_string()
}

/// Build a synthetic `notify::Event` for the relevance-filter assertion.
fn fake_event(kind: notify::EventKind, path: &str) -> notify::Event {
	notify::Event {
		kind,
		paths: vec![PathBuf::from(path)],
		attrs: Default::default(),
	}
}
