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
	let prev = std::env::current_dir().unwrap();
	std::env::set_current_dir(tmp.path()).unwrap();

	// Act
	let res = StartProjectCommand
		.execute(&pages_context(vec!["polls_project".to_string()]))
		.await;

	// Assert
	std::env::set_current_dir(prev).unwrap();
	res.expect("startproject --with-pages must succeed");

	let project = tmp.path().join("polls_project");
	let src = project.join("src");

	// 1. server_fn lives at the crate root, NOT under server/.
	assert!(
		src.join("server_fn.rs").exists(),
		"src/server_fn.rs must exist at the crate root"
	);
	assert!(
		!src.join("server").exists(),
		"src/server/ must not be generated (server_fn moved to crate root)"
	);

	// 2. config/wasm.rs is required for collectstatic to find dist-wasm.
	assert!(
		src.join("config").join("wasm.rs").exists(),
		"src/config/wasm.rs must exist (collectstatic registration)"
	);

	// 3. client/lib.rs (WASM entry) and client/pages.rs replace
	//    bootstrap/state from the previous scaffold.
	assert!(
		src.join("client").join("lib.rs").exists(),
		"src/client/lib.rs must exist (WASM entry point)"
	);
	assert!(
		src.join("client").join("pages.rs").exists(),
		"src/client/pages.rs must exist (page components)"
	);
	assert!(
		src.join("client").join("router.rs").exists(),
		"src/client/router.rs must exist"
	);
	assert!(
		src.join("client").join("components.rs").exists(),
		"src/client/components.rs must exist"
	);
	assert!(
		!src.join("client").join("bootstrap.rs").exists(),
		"src/client/bootstrap.rs must not be generated"
	);
	assert!(
		!src.join("client").join("state.rs").exists(),
		"src/client/state.rs must not be generated"
	);

	// 4. lib.rs declares the server-only re-export shim.
	let lib_rs = fs::read_to_string(src.join("lib.rs")).expect("read lib.rs");
	assert!(
		lib_rs.contains("mod server_only"),
		"src/lib.rs must declare the `mod server_only` re-export shim:\n{lib_rs}"
	);
	assert!(
		lib_rs.contains("pub mod server_fn;"),
		"src/lib.rs must declare `pub mod server_fn;` (top-level)"
	);

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
	let prev = std::env::current_dir().unwrap();
	std::env::set_current_dir(tmp).unwrap();
	let cmd = StartProjectCommand;
	let ctx = pages_context(vec![name.to_string()]);
	let result = cmd.execute(&ctx).await;
	std::env::set_current_dir(prev).unwrap();
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
	let prev = std::env::current_dir().unwrap();
	std::env::set_current_dir(&project_dir).unwrap();

	// Act
	let res = StartAppCommand
		.execute(&pages_context(vec!["polls".to_string()]))
		.await;

	// Assert
	std::env::set_current_dir(prev).unwrap();
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

	// 3. None of the previous over-generated subdirectories are produced.
	//    `urls/ws_urls.rs` remains forbidden (websocket scaffold is opt-in
	//    and explicitly out of scope for #4308). `urls/server_urls.rs` and
	//    `urls/client_router.rs` are now REQUIRED by the canonical layout
	//    introduced in rc.19 (see `startapp_pages_layout_has_urls_submodule`
	//    below for the positive assertions).
	for unwanted in ["client", "server", "shared", "urls/ws_urls.rs"] {
		let path = polls_dir.join(unwanted);
		assert!(
			!path.exists(),
			"apps/polls/{unwanted} must not be generated (was: {})",
			path.display()
		);
	}

	// 4. urls.rs is the aggregator that declares the `server_urls` and
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
	let prev = std::env::current_dir().unwrap();
	std::env::set_current_dir(&project_dir).unwrap();

	// Act
	let res = StartAppCommand
		.execute(&pages_context(vec!["foo".to_string()]))
		.await;

	// Assert
	std::env::set_current_dir(prev).unwrap();
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

	// 4. The submodules carry the canonical #[url_patterns] attribute with
	//    `mode = server` / `mode = client`, matching the polls tutorial.
	let server_contents = fs::read_to_string(&server_urls).expect("read server_urls.rs");
	assert!(
		server_contents.contains("#[url_patterns(InstalledApp::foo, mode = server)]"),
		"server_urls.rs must carry the server-mode #[url_patterns] attribute:\n{server_contents}"
	);

	let client_contents = fs::read_to_string(&client_router).expect("read client_router.rs");
	assert!(
		client_contents.contains("#[url_patterns(InstalledApp::foo, mode = client)]"),
		"client_router.rs must carry the client-mode #[url_patterns] attribute:\n{client_contents}"
	);
}
