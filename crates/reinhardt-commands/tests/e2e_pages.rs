//! End-to-end checks that `startproject --with-pages` and
//! `startapp --with-pages` produce the directory layout taught in the
//! Reinhardt basics tutorial (`examples/examples-tutorial-basis/`).
//!
//! These tests guard against regressions of #3970 — every assertion below
//! corresponds to a concrete divergence reported in that issue.

use reinhardt_commands::start_commands::{StartAppCommand, StartProjectCommand};
use reinhardt_commands::{BaseCommand, CommandContext};
use rstest::*;
use serial_test::serial;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// RAII guard that restores the process-wide current working directory when
/// dropped, including on panic-driven unwind. Tests under `#[serial(cwd)]`
/// mutate global state via `std::env::set_current_dir`, so without this guard
/// a panic inside `execute(...)` would leave the CWD pointing at a `TempDir`
/// that gets deleted at end-of-scope, corrupting subsequent tests.
struct CwdGuard {
	prev: std::path::PathBuf,
}

impl CwdGuard {
	fn enter(new_cwd: &Path) -> Self {
		let prev = std::env::current_dir().unwrap();
		std::env::set_current_dir(new_cwd).unwrap();
		Self { prev }
	}
}

impl Drop for CwdGuard {
	fn drop(&mut self) {
		// Best-effort restore — swallow errors during unwind so we never
		// double-panic. The original directory may itself have been removed
		// in pathological cases, which is acceptable for test cleanup.
		let _ = std::env::set_current_dir(&self.prev);
	}
}

/// Helper: build a `CommandContext` whose only option is `--with-pages`.
fn pages_context(args: Vec<String>) -> CommandContext {
	let mut ctx = CommandContext::new(args);
	let mut opts = HashMap::new();
	opts.insert("with-pages".to_string(), vec!["true".to_string()]);
	ctx = ctx.with_options(opts);
	ctx
}

#[rstest]
#[tokio::test]
#[serial(cwd)]
async fn project_pages_layout_matches_tutorial() {
	// Arrange
	let tmp = TempDir::new().unwrap();
	let _cwd_guard = CwdGuard::enter(tmp.path());

	// Act
	let res = StartProjectCommand
		.execute(&pages_context(vec!["polls_project".to_string()]))
		.await;

	// Assert
	res.expect("startproject --with-pages must succeed");

	let project = tmp.path().join("polls_project");
	let src = project.join("src");

	// 1. Per-app reorg: top-level `src/server_fn.rs` no longer exists; each
	//    app owns its own under `apps/<app>/server_fn.rs`.
	assert!(
		!src.join("server_fn.rs").exists(),
		"src/server_fn.rs must NOT exist (per-app under apps/<app>/server_fn.rs)"
	);
	assert!(
		!src.join("server_fn").exists(),
		"src/server_fn/ directory must NOT exist"
	);
	assert!(
		!src.join("server").exists(),
		"src/server/ must not be generated"
	);

	// 2. config/wasm.rs is required for collectstatic to find dist-wasm.
	assert!(
		src.join("config").join("wasm.rs").exists(),
		"src/config/wasm.rs must exist (collectstatic registration)"
	);

	// 3. Client shell: lib.rs (WASM entry) + components.rs (`pub mod nav;`
	//    only). Per-app pages and components live under `apps/<app>/client/`.
	assert!(
		src.join("client").join("lib.rs").exists(),
		"src/client/lib.rs must exist (WASM entry point)"
	);
	assert!(
		src.join("client").join("components.rs").exists(),
		"src/client/components.rs must exist (shared shell aggregator)"
	);
	assert!(
		src.join("client")
			.join("components")
			.join("nav.rs")
			.exists(),
		"src/client/components/nav.rs must exist (shared with_nav helper)"
	);
	assert!(
		!src.join("client").join("pages.rs").exists(),
		"src/client/pages.rs must NOT exist (per-app under apps/<app>/client/pages.rs)"
	);
	assert!(
		!src.join("client").join("router.rs").exists(),
		"src/client/router.rs must NOT exist (replaced by per-app client_router.rs)"
	);
	assert!(
		!src.join("client").join("bootstrap.rs").exists(),
		"src/client/bootstrap.rs must not be generated"
	);
	assert!(
		!src.join("client").join("state.rs").exists(),
		"src/client/state.rs must not be generated"
	);

	// 4. lib.rs declares the server-only re-export shim and un-gates apps.
	let lib_rs = fs::read_to_string(src.join("lib.rs")).expect("read lib.rs");
	assert!(
		lib_rs.contains("mod server_only"),
		"src/lib.rs must declare the `mod server_only` re-export shim:\n{lib_rs}"
	);
	assert!(
		!lib_rs.contains("pub mod server_fn;"),
		"src/lib.rs must NOT declare `pub mod server_fn;` (moved per-app):\n{lib_rs}"
	);
	// `pub mod apps;` must be un-gated (no `#[cfg(server)]` directly above it).
	for cfg_line in [
		"#[cfg(server)]\npub mod apps;",
		"#[cfg(not(client))]\npub mod apps;",
	] {
		assert!(
			!lib_rs.contains(cfg_line),
			"src/lib.rs must declare `pub mod apps;` without a cfg gate (found {cfg_line:?}):\n{lib_rs}"
		);
	}
	assert!(
		lib_rs.contains("pub mod apps;"),
		"src/lib.rs must still declare `pub mod apps;`:\n{lib_rs}"
	);

	// 4b. The shared components shell aggregator declares only `pub mod nav;`.
	let components_rs = fs::read_to_string(src.join("client").join("components.rs"))
		.expect("read client/components.rs");
	assert!(
		components_rs.contains("pub mod nav;"),
		"src/client/components.rs must declare `pub mod nav;`:\n{components_rs}"
	);
	for unwanted in ["pub mod polls", "pub mod users"] {
		assert!(
			!components_rs.contains(unwanted),
			"src/client/components.rs must not preserve per-app submodules ({unwanted}):\n{components_rs}"
		);
	}

	// 5. shared/ now exposes types and forms (no more shared/errors.rs).
	assert!(
		src.join("shared").join("types.rs").exists(),
		"src/shared/types.rs must exist"
	);
	assert!(
		src.join("shared").join("forms.rs").exists(),
		"src/shared/forms.rs must exist (Part 4 forms scaffold)"
	);
	assert!(
		!src.join("shared").join("errors.rs").exists(),
		"src/shared/errors.rs must not be generated"
	);

	// 6. Part 5 (Testing) directory is in place.
	assert!(
		project.join("tests").join("integration.rs").exists(),
		"tests/integration.rs must exist (Part 5 testing scaffold)"
	);
	assert!(
		project.join("tests").join("wasm").exists(),
		"tests/wasm/ must exist (Part 5 WASM tests directory)"
	);
}

/// Set up a project so that `startapp --with-pages` can run inside it.
async fn scaffold_pages_project(tmp: &Path, name: &str) {
	let _cwd_guard = CwdGuard::enter(tmp);
	let cmd = StartProjectCommand;
	let ctx = pages_context(vec![name.to_string()]);
	let result = cmd.execute(&ctx).await;
	result.expect("startproject --with-pages must succeed");
}

#[rstest]
#[tokio::test]
#[serial(cwd)]
async fn app_pages_layout_matches_tutorial() {
	// Arrange
	let tmp = TempDir::new().unwrap();
	let project_name = "polls_project";
	scaffold_pages_project(tmp.path(), project_name).await;

	let project_dir = tmp.path().join(project_name);
	let _cwd_guard = CwdGuard::enter(&project_dir);

	// Act
	let res = StartAppCommand
		.execute(&pages_context(vec!["polls".to_string()]))
		.await;

	// Assert
	res.expect("startapp --with-pages must succeed");

	let apps = project_dir.join("src").join("apps");

	// 1. apps/<app>.rs is the module entry point (Rust 2024 edition).
	assert!(
		apps.join("polls.rs").exists(),
		"src/apps/polls.rs must exist as the app's module entry point"
	);
	let polls_rs = fs::read_to_string(apps.join("polls.rs")).expect("read apps/polls.rs");
	assert!(
		polls_rs.contains("#[app_config(name = \"polls\", label = \"polls\")]"),
		"apps/polls.rs must carry the #[app_config] attribute:\n{polls_rs}"
	);

	// 2. Sub-modules sit directly under apps/<app>/, mirroring the tutorial.
	let polls_dir = apps.join("polls");
	assert!(
		polls_dir.join("models.rs").exists(),
		"apps/polls/models.rs must exist"
	);
	assert!(
		polls_dir.join("serializers.rs").exists(),
		"apps/polls/serializers.rs must exist"
	);
	assert!(
		polls_dir.join("urls.rs").exists(),
		"apps/polls/urls.rs must exist"
	);
	assert!(
		polls_dir.join("views.rs").exists(),
		"apps/polls/views.rs must exist"
	);

	// 3. `server/`, `shared/`, and `urls/ws_urls.rs` remain forbidden.
	//    `client/` is now REQUIRED at the per-app level (per-app UI lives
	//    here). `urls/server_urls.rs` and `urls/client_router.rs` are also
	//    REQUIRED (see `startapp_pages_layout_has_urls_submodule` and
	//    `startapp_pages_layout_per_app_modules` for positive assertions).
	for unwanted in ["server", "shared", "urls/ws_urls.rs"] {
		let path = polls_dir.join(unwanted);
		assert!(
			!path.exists(),
			"apps/polls/{unwanted} must not be generated (was: {})",
			path.display()
		);
	}

	// 4. Per-app server_fn / client.rs / client/{components,pages}.rs and
	//    server_fn placeholder are generated.
	assert!(
		polls_dir.join("server_fn.rs").exists(),
		"apps/polls/server_fn.rs must exist"
	);
	let server_fn_rs =
		fs::read_to_string(polls_dir.join("server_fn.rs")).expect("read apps/polls/server_fn.rs");
	assert!(
		server_fn_rs.contains("#[server_fn]") && server_fn_rs.contains("pub async fn placeholder"),
		"apps/polls/server_fn.rs must contain a #[server_fn]-annotated placeholder:\n{server_fn_rs}"
	);

	assert!(
		polls_dir.join("client.rs").exists(),
		"apps/polls/client.rs must exist"
	);
	let client_rs =
		fs::read_to_string(polls_dir.join("client.rs")).expect("read apps/polls/client.rs");
	assert!(
		client_rs.contains("pub mod components;") && client_rs.contains("pub mod pages;"),
		"apps/polls/client.rs must declare `pub mod components;` and `pub mod pages;`:\n{client_rs}"
	);

	let polls_components = polls_dir.join("client").join("components.rs");
	let polls_pages = polls_dir.join("client").join("pages.rs");
	assert!(
		polls_components.exists(),
		"apps/polls/client/components.rs must exist"
	);
	assert!(
		polls_pages.exists(),
		"apps/polls/client/pages.rs must exist"
	);
	let components_body = fs::read_to_string(&polls_components).expect("read components.rs");
	assert!(
		components_body.contains("pub fn placeholder"),
		"apps/polls/client/components.rs must declare `pub fn placeholder`:\n{components_body}"
	);
	let pages_body = fs::read_to_string(&polls_pages).expect("read pages.rs");
	assert!(
		pages_body.contains("pub fn placeholder_page") && pages_body.contains("with_nav"),
		"apps/polls/client/pages.rs must declare `pub fn placeholder_page` and call `with_nav`:\n{pages_body}"
	);

	// 5. urls.rs is the aggregator that declares the `server_urls` and
	//    `client_router` submodules with the appropriate cfg gates. The
	//    legacy `unified_url_patterns` scaffold is still forbidden.
	let urls_rs = fs::read_to_string(polls_dir.join("urls.rs")).expect("read apps/polls/urls.rs");
	assert!(
		urls_rs.contains("pub mod server_urls"),
		"apps/polls/urls.rs must declare `pub mod server_urls`:\n{urls_rs}"
	);
	assert!(
		urls_rs.contains("pub mod client_router"),
		"apps/polls/urls.rs must declare `pub mod client_router`:\n{urls_rs}"
	);
	assert!(
		!urls_rs.contains("unified_url_patterns"),
		"apps/polls/urls.rs must not use the unified_url_patterns scaffold:\n{urls_rs}"
	);
}

#[rstest]
#[tokio::test]
#[serial(cwd)]
async fn startapp_pages_layout_has_urls_submodule() {
	// Arrange — scaffold a project, then scaffold a pages app inside it.
	let tmp = TempDir::new().unwrap();
	let project_name = "polls_project";
	scaffold_pages_project(tmp.path(), project_name).await;

	let project_dir = tmp.path().join(project_name);
	let _cwd_guard = CwdGuard::enter(&project_dir);

	// Act
	let res = StartAppCommand
		.execute(&pages_context(vec!["foo".to_string()]))
		.await;

	// Assert
	res.expect("startapp --with-pages must succeed");

	let foo_dir = project_dir.join("src").join("apps").join("foo");

	// 1. The aggregator and both submodules exist.
	let urls_rs = foo_dir.join("urls.rs");
	let server_urls = foo_dir.join("urls").join("server_urls.rs");
	let client_router = foo_dir.join("urls").join("client_router.rs");
	assert!(urls_rs.exists(), "apps/foo/urls.rs must exist");
	assert!(
		server_urls.exists(),
		"apps/foo/urls/server_urls.rs must exist"
	);
	assert!(
		client_router.exists(),
		"apps/foo/urls/client_router.rs must exist"
	);

	// 2. ws_urls.rs is explicitly out of scope (see #4308 scope boundaries).
	assert!(
		!foo_dir.join("urls").join("ws_urls.rs").exists(),
		"apps/foo/urls/ws_urls.rs must NOT be generated"
	);

	// 3. The aggregator declares both submodules with the appropriate cfg
	//    gates that match the project-level `build.rs` cfg_aliases
	//    (`client` for wasm32, `server` for native).
	let urls_contents = fs::read_to_string(&urls_rs).expect("read apps/foo/urls.rs");
	assert!(
		urls_contents.contains("#[cfg(server)]"),
		"apps/foo/urls.rs must gate server_urls with #[cfg(server)]:\n{urls_contents}"
	);
	assert!(
		urls_contents.contains("pub mod server_urls"),
		"apps/foo/urls.rs must declare `pub mod server_urls`:\n{urls_contents}"
	);
	assert!(
		urls_contents.contains("#[cfg(client)]"),
		"apps/foo/urls.rs must gate client_router with #[cfg(client)]:\n{urls_contents}"
	);
	assert!(
		urls_contents.contains("pub mod client_router"),
		"apps/foo/urls.rs must declare `pub mod client_router`:\n{urls_contents}"
	);

	// 4. Sub-routers carry the canonical #[url_patterns] attribute, and their
	//    function bodies are empty. The bodies are isolated from the module
	//    doc-comment (which may legitimately quote an example) by slicing the
	//    file at the `pub fn` definition before searching for example calls.
	let server_contents = fs::read_to_string(&server_urls).expect("read server_urls.rs");
	assert!(
		server_contents.contains("#[url_patterns(InstalledApp::foo, mode = server)]"),
		"server_urls.rs must carry the server-mode #[url_patterns] attribute:\n{server_contents}"
	);
	let server_body_start = server_contents
		.find("pub fn server_url_patterns")
		.expect("server_urls.rs must define `pub fn server_url_patterns`");
	let server_body = &server_contents[server_body_start..];
	assert!(
		!server_body.contains(".endpoint(views::"),
		"server_urls.rs function body must not embed example route calls:\n{server_body}"
	);

	let client_contents = fs::read_to_string(&client_router).expect("read client_router.rs");
	assert!(
		client_contents.contains("#[url_patterns(InstalledApp::foo, mode = client)]"),
		"client_router.rs must carry the client-mode #[url_patterns] attribute:\n{client_contents}"
	);
	let client_body_start = client_contents
		.find("pub fn client_url_patterns")
		.expect("client_router.rs must define `pub fn client_url_patterns`");
	let client_body = &client_contents[client_body_start..];
	assert!(
		!client_body.contains(".route(\"index\","),
		"client_router.rs function body must not embed example route calls:\n{client_body}"
	);

	// 5. Per-app aggregator `apps/foo.rs` declares `#[cfg(client)] pub mod client;`
	//    and bi-target `pub mod server_fn;` / `pub mod urls;` without cfg gates.
	let foo_rs = fs::read_to_string(foo_dir.parent().expect("apps/").join("foo.rs"))
		.expect("read apps/foo.rs");
	assert!(
		foo_rs.contains("#[cfg(client)]\npub mod client;"),
		"apps/foo.rs must declare `#[cfg(client)] pub mod client;`:\n{foo_rs}"
	);
	// Bi-target lines: ensure they have no cfg attr immediately preceding.
	for bi_target in ["pub mod server_fn;", "pub mod urls;"] {
		let pos = foo_rs
			.find(bi_target)
			.unwrap_or_else(|| panic!("`{bi_target}` not found in apps/foo.rs:\n{foo_rs}"));
		let prefix = &foo_rs[..pos];
		let prior_line = prefix.lines().last().unwrap_or("").trim();
		assert!(
			!prior_line.starts_with("#[cfg("),
			"`{bi_target}` must not be cfg-gated in apps/foo.rs (preceding line was: {prior_line:?}):\n{foo_rs}"
		);
	}
}
