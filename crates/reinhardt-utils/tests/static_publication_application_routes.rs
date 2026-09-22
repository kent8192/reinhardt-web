#![cfg(feature = "asset-publication")]

use async_trait::async_trait;
use http::{Method, StatusCode, header};
use reinhardt_http::{Handler, Middleware, Request, Response};
use reinhardt_utils::staticfiles::publication::{
	AssetInput, AssetMode, AssetPipeline, AssetProducer, AssetPublisher, ManifestServingConfig,
	ManifestStaticMiddleware, ManifestStore, PagesEntrypoint, SnapshotOptions,
};
use rstest::rstest;
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

struct ApplicationRoute {
	calls: AtomicUsize,
	status: StatusCode,
}

#[async_trait]
impl Handler for ApplicationRoute {
	async fn handle(&self, _: Request) -> reinhardt_core::exception::Result<Response> {
		self.calls.fetch_add(1, Ordering::SeqCst);
		Ok(Response::new(self.status)
			.with_header("content-type", "application/octet-stream")
			.with_header("x-application-route", "preserved")
			.with_body("application response"))
	}
}

#[rstest]
#[case("/service-worker.js", Method::GET, StatusCode::OK)]
#[case("/service-worker.js", Method::HEAD, StatusCode::OK)]
#[case("/downloads/theme.css", Method::GET, StatusCode::PARTIAL_CONTENT)]
#[case("/modules/runtime.mjs", Method::GET, StatusCode::OK)]
#[case("/modules/vendor.cjs", Method::GET, StatusCode::OK)]
#[case("/modules/plugin.wasm", Method::GET, StatusCode::OK)]
#[case("/downloads/THEME.CSS", Method::GET, StatusCode::OK)]
#[case("/missing.js", Method::GET, StatusCode::NOT_FOUND)]
#[case("/missing.css", Method::HEAD, StatusCode::NOT_FOUND)]
#[tokio::test]
async fn asset_like_paths_outside_mount_preserve_application_responses(
	#[case] path: &str,
	#[case] method: Method,
	#[case] status: StatusCode,
) {
	// Arrange: enable navigation so an application 404 would otherwise render HTML.
	let root = tempfile::tempdir().unwrap();
	let mut pipeline = AssetPipeline::new();
	pipeline
		.add_input(
			AssetInput::bytes(
				"app.js",
				b"export default function init(){};export const wasm=new URL('app_bg.wasm',import.meta.url);".to_vec(),
			)
			.with_producer(AssetProducer::Pages),
		)
		.unwrap();
	pipeline
		.add_input(
			AssetInput::bytes("app_bg.wasm", b"\0asm\x01\0\0\0".to_vec())
				.with_producer(AssetProducer::Pages),
		)
		.unwrap();
	pipeline
		.set_entrypoint(
			"default",
			PagesEntrypoint {
				javascript: "app.js".into(),
				wasm: "app_bg.wasm".into(),
				styles: Vec::new(),
				document: None,
			},
		)
		.unwrap();
	AssetPublisher::new(root.path().into())
		.publish(pipeline.prepare(AssetMode::Production).unwrap())
		.unwrap();
	let store = Arc::new(
		ManifestStore::open(root.path().into(), SnapshotOptions::production()).unwrap(),
	);
	let config = ManifestServingConfig::new(store, "/static/".into())
		.unwrap()
		.with_navigation_fallback(true);
	config.validate().unwrap();
	let middleware = ManifestStaticMiddleware::new(config);
	let application = Arc::new(ApplicationRoute {
		calls: AtomicUsize::new(0),
		status,
	});

	// Act
	let response = middleware
		.process(
			Request::builder()
				.uri(path)
				.method(method)
				.header(header::ACCEPT, "text/html")
				.build()
				.unwrap(),
			application.clone(),
		)
		.await
		.unwrap();

	// Assert: neither a synthetic static 404 nor a navigation shell may replace it.
	assert_eq!(application.calls.load(Ordering::SeqCst), 1);
	assert_eq!(response.status, status);
	assert_eq!(response.headers["x-application-route"], "preserved");
	assert_eq!(response.headers[header::CONTENT_TYPE], "application/octet-stream");
	assert_eq!(response.body.as_ref(), b"application response");
}
