//! Integration tests for the project-static auto-mount behavior introduced
//! by Issue #4484.
//!
//! These tests exercise the *cascade* of two `StaticFilesMiddleware`
//! instances that `runserver --with-pages` registers when a
//! `<project-root>/static/` directory is present:
//!
//! 1. A project-static middleware mounted at `/static/` with
//!    `spa_mode = false` and `passthrough_prefixes = ["/static/admin/"]`.
//! 2. The existing dist middleware mounted at `/` (covers WASM bundle and
//!    SPA fallback) and excluding `/static/admin/` from SPA fallback.
//!
//! Anything that neither middleware serves falls through to the application
//! router. The tests fake the application router with a small handler so the
//! cascade can be exercised end-to-end through a real `HttpServer`.

use super::server_test_helpers::{shutdown_test_server, spawn_test_server};
use async_trait::async_trait;
use reinhardt_core::exception::Result;
use reinhardt_http::{Handler, MiddlewareChain, Request, Response};
use reinhardt_utils::staticfiles::{StaticFilesMiddleware, StaticMiddlewareConfig};
use std::sync::Arc;
use tempfile::TempDir;

/// Handler that simulates the application router's `/static/admin/...` mount
/// (provided by `admin_static_routes()` in production). It returns a sentinel
/// body so the test can confirm passthrough reached the router.
struct AdminAssetRouter;

#[async_trait]
impl Handler for AdminAssetRouter {
	async fn handle(&self, request: Request) -> Result<Response> {
		let path = request.uri.path();
		if let Some(rest) = path.strip_prefix("/static/admin/") {
			Ok(Response::ok()
				.with_header("Content-Type", "text/css")
				.with_body(format!("/* admin-bundle:{} */", rest)))
		} else {
			Ok(Response::not_found().with_body("not found"))
		}
	}
}

/// Build the cascade as `runserver --with-pages` does: project-static added
/// first (outermost in the `MiddlewareChain`), dist-static second (inner),
/// final handler = mock application router.
fn build_runserver_cascade(
	project_static_dir: std::path::PathBuf,
	dist_dir: std::path::PathBuf,
) -> Arc<dyn Handler> {
	let project_static = Arc::new(StaticFilesMiddleware::new(
		StaticMiddlewareConfig::new(project_static_dir)
			.url_prefix("/static/")
			.spa_mode(false)
			.auto_inject_wasm(false)
			.passthrough_prefixes(vec!["/static/admin/".to_string()]),
	));
	let dist_static = Arc::new(StaticFilesMiddleware::new(
		StaticMiddlewareConfig::new(dist_dir)
			.url_prefix("/")
			.spa_mode(true)
			.excluded_prefixes(vec![
				"/api/".to_string(),
				"/admin/".to_string(),
				"/static/admin/".to_string(),
			]),
	));
	// MiddlewareChain::handle iterates `.iter().rev()` at request time, so
	// the first `.with_middleware` becomes the outermost layer and executes
	// first (project_static), then falls through to dist_static, then to the
	// base handler. The reversal happens during request handling, not chain
	// construction.
	let chain = MiddlewareChain::new(Arc::new(AdminAssetRouter))
		.with_middleware(project_static)
		.with_middleware(dist_static);
	Arc::new(chain)
}

#[tokio::test]
async fn test_project_static_serves_project_css() {
	// Arrange — project-root layout with hand-written CSS plus a dist/
	// containing only the SPA shell.
	let project = TempDir::new().unwrap();
	let project_static = project.path().join("static");
	std::fs::create_dir_all(project_static.join("css")).unwrap();
	std::fs::write(
		project_static.join("css/style.css"),
		"body { color: rebeccapurple; }",
	)
	.unwrap();
	let dist = project.path().join("dist");
	std::fs::create_dir_all(&dist).unwrap();
	std::fs::write(dist.join("index.html"), "<html><body>spa</body></html>").unwrap();

	let handler = build_runserver_cascade(project_static, dist);
	let (url, server_handle) = spawn_test_server(handler).await;

	// Act
	let response = reqwest::get(format!("{}/static/css/style.css", url))
		.await
		.expect("request failed");

	// Assert
	assert_eq!(response.status(), 200);
	let content_type = response
		.headers()
		.get("content-type")
		.expect("missing Content-Type")
		.to_str()
		.unwrap()
		.to_owned();
	assert!(
		content_type.contains("text/css"),
		"expected text/css, got {content_type}"
	);
	let body = response.text().await.unwrap();
	assert_eq!(body, "body { color: rebeccapurple; }");

	shutdown_test_server(server_handle).await;
}

#[tokio::test]
async fn test_admin_static_wins_over_project_collision() {
	// Arrange — project mistakenly contains static/admin/foo.css; the
	// project-static middleware must skip the path so the admin router wins.
	let project = TempDir::new().unwrap();
	let project_static = project.path().join("static");
	std::fs::create_dir_all(project_static.join("admin")).unwrap();
	std::fs::write(
		project_static.join("admin/foo.css"),
		"/* user-supplied collision */",
	)
	.unwrap();
	let dist = project.path().join("dist");
	std::fs::create_dir_all(&dist).unwrap();
	std::fs::write(dist.join("index.html"), "<html></html>").unwrap();

	let handler = build_runserver_cascade(project_static, dist);
	let (url, server_handle) = spawn_test_server(handler).await;

	// Act
	let response = reqwest::get(format!("{}/static/admin/foo.css", url))
		.await
		.expect("request failed");

	// Assert
	assert_eq!(response.status(), 200);
	let body = response.text().await.unwrap();
	assert_eq!(
		body, "/* admin-bundle:foo.css */",
		"admin router must win over a colliding file in <project>/static/admin/"
	);

	shutdown_test_server(server_handle).await;
}

#[tokio::test]
async fn test_dist_assets_still_served_alongside_project_static() {
	// Arrange — dist/ has WASM bundle outputs; project-static must not
	// shadow them since they live at `/` not `/static/`.
	let project = TempDir::new().unwrap();
	let project_static = project.path().join("static");
	std::fs::create_dir_all(&project_static).unwrap();
	let dist = project.path().join("dist");
	std::fs::create_dir_all(&dist).unwrap();
	std::fs::write(dist.join("app.js"), "// bundled js").unwrap();
	std::fs::write(dist.join("app_bg.wasm"), b"\0asm\x01\x00\x00\x00").unwrap();
	std::fs::write(
		dist.join("index.html"),
		"<html><body>spa shell</body></html>",
	)
	.unwrap();

	let handler = build_runserver_cascade(project_static, dist);
	let (url, server_handle) = spawn_test_server(handler).await;

	// Act
	let js = reqwest::get(format!("{}/app.js", url))
		.await
		.expect("request failed");
	let spa = reqwest::get(format!("{}/some/spa/route", url))
		.await
		.expect("request failed");

	// Assert
	assert_eq!(js.status(), 200);
	assert_eq!(js.text().await.unwrap(), "// bundled js");
	assert_eq!(spa.status(), 200);
	assert!(
		spa.text().await.unwrap().contains("spa shell"),
		"unknown SPA route must fall back to dist/index.html"
	);

	shutdown_test_server(server_handle).await;
}

#[tokio::test]
async fn test_project_static_missing_falls_through_to_dist_spa_fallback() {
	// Arrange — request for a file that exists in neither project-static
	// nor dist. SPA mode is OFF on project-static, so it must fall through;
	// dist's SPA fallback returns index.html for non-excluded paths.
	let project = TempDir::new().unwrap();
	let project_static = project.path().join("static");
	std::fs::create_dir_all(&project_static).unwrap();
	let dist = project.path().join("dist");
	std::fs::create_dir_all(&dist).unwrap();
	std::fs::write(dist.join("index.html"), "<html><body>spa</body></html>").unwrap();

	let handler = build_runserver_cascade(project_static, dist);
	let (url, server_handle) = spawn_test_server(handler).await;

	// Act
	let response = reqwest::get(format!("{}/static/missing.css", url))
		.await
		.expect("request failed");

	// Assert — Issue #4484: project-static must NOT silently serve missing
	// `/static/...` files with an empty 200; it must fall through so dist's
	// SPA fallback (the pre-#4484 behaviour for paths outside
	// `/static/admin/`) can serve `index.html` as before. The expected
	// observable response here is therefore the SPA HTML coming from dist.
	assert_eq!(response.status(), 200);
	let content_type = response
		.headers()
		.get("content-type")
		.expect("missing Content-Type")
		.to_str()
		.unwrap()
		.to_owned();
	assert!(
		content_type.contains("text/html"),
		"expected SPA fallback HTML, got Content-Type {content_type}"
	);
	// Confirm the body is actually dist/index.html (not e.g. an empty 200
	// from project-static or a wrong fallback). Exact match guards against
	// future regressions that silently substitute the SPA shell.
	let body = response.text().await.unwrap();
	assert_eq!(
		body, "<html><body>spa</body></html>",
		"SPA fallback should serve dist/index.html verbatim"
	);

	shutdown_test_server(server_handle).await;
}
