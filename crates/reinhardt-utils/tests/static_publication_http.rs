#![cfg(feature = "asset-publication")]

use async_trait::async_trait;
use http::{Method, StatusCode, header};
use reinhardt_http::{Handler, Middleware, Request, Response};
use reinhardt_utils::staticfiles::publication::{
	AssetInput, AssetMode, AssetPipeline, AssetProducer, AssetPublisher, AssetRole,
	ManifestServingConfig, ManifestStaticMiddleware, ManifestStore, PagesEntrypoint,
	SnapshotOptions,
};
use rstest::rstest;
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

struct NavigationProbe {
	calls: AtomicUsize,
	status: StatusCode,
}

impl NavigationProbe {
	fn new(status: StatusCode) -> Arc<Self> {
		Arc::new(Self {
			calls: AtomicUsize::new(0),
			status,
		})
	}
	fn call_count(&self) -> usize {
		self.calls.load(Ordering::SeqCst)
	}
}

#[async_trait]
impl Handler for NavigationProbe {
	async fn handle(&self, _: Request) -> reinhardt_core::exception::Result<Response> {
		self.calls.fetch_add(1, Ordering::SeqCst);
		Ok(Response::new(self.status)
			.with_header("content-type", "text/html")
			.with_body("navigation-probe"))
	}
}

struct Fixture {
	_root: tempfile::TempDir,
	store: Arc<ManifestStore>,
	middleware: ManifestStaticMiddleware,
	prefix: String,
}

impl Fixture {
	fn new(prefix: &str, mode: AssetMode) -> Self {
		let root = tempfile::tempdir().unwrap();
		let mut pipeline = AssetPipeline::new();
		for (name, content) in [("app.js", &b"export default function init(){};export const wasm=new URL('app_bg.wasm',import.meta.url);"[..]), ("app_bg.wasm", b"\0asm\x01\0\0\0")] {
			pipeline.add_input(AssetInput::bytes(name, content.into()).with_producer(AssetProducer::Pages)).unwrap();
		}
		pipeline
			.add_input(AssetInput::bytes(
				"site.css",
				b"body{color:rgb(1,2,3)}".into(),
			))
			.unwrap();
		pipeline
			.add_input(AssetInput::bytes("logo.svg", b"<svg/>".into()))
			.unwrap();
		pipeline.add_input(AssetInput::bytes("index.html", br#"<!doctype html><html><head><link rel="stylesheet" href="{{ static_url('site.css') }}"></head><body><img src="logo.svg"><script type="module">const image = new URL('logo.svg',import.meta.url);</script></body></html>"#.to_vec()).with_role(AssetRole::EntryDocument)).unwrap();
		pipeline
			.set_entrypoint(
				"default",
				PagesEntrypoint {
					javascript: "app.js".into(),
					wasm: "app_bg.wasm".into(),
					styles: vec!["site.css".into()],
					document: Some("index.html".into()),
				},
			)
			.unwrap();
		AssetPublisher::new(root.path().into())
			.publish(pipeline.prepare(mode).unwrap())
			.unwrap();
		let options = match mode {
			AssetMode::Production => SnapshotOptions::production(),
			AssetMode::Development => SnapshotOptions::development(),
		};
		let store = Arc::new(ManifestStore::open(root.path().into(), options).unwrap());
		let config = ManifestServingConfig::new(store.clone(), prefix.into())
			.unwrap()
			.with_pages("default".into())
			.with_navigation_fallback(true);
		Self {
			_root: root,
			store,
			middleware: ManifestStaticMiddleware::new(config),
			prefix: prefix.into(),
		}
	}
	fn url(&self, logical: &str) -> String {
		self.store
			.active()
			.url_snapshot(&self.prefix)
			.unwrap()
			.resolve(logical)
			.unwrap()
	}
}

#[rstest]
#[case("/static/", "pages/missing.js")]
#[case("/console/static/", "pages/missing.wasm")]
#[case("/", "css/missing.css")]
#[case("https://cdn.example.test/assets/", "other/extensionless")]
#[tokio::test]
async fn missing_generation_assets_never_enter_navigation(
	#[case] prefix: &str,
	#[case] missing: &str,
) {
	// Arrange
	let fixture = Fixture::new(prefix, AssetMode::Production);
	let probe = NavigationProbe::new(StatusCode::OK);
	let uri = format!(
		"{prefix}builds/{}/{missing}",
		fixture.store.active().manifest().build_id
	);
	// Act
	let response = fixture
		.middleware
		.process(Request::builder().uri(&uri).build().unwrap(), probe.clone())
		.await
		.unwrap();
	// Assert
	assert_eq!(response.status, StatusCode::NOT_FOUND);
	assert_ne!(response.headers[header::CONTENT_TYPE], "text/html");
	assert_eq!(response.headers[header::CACHE_CONTROL], "no-store");
	assert_eq!(probe.call_count(), 0);
}

#[rstest]
#[case("/static/")]
#[case("/console/static/")]
#[case("/")]
#[case("https://cdn.example.test/console/static/")]
#[tokio::test]
async fn nested_navigation_uses_full_snapshot_urls_before_module_evaluation(#[case] prefix: &str) {
	// Arrange
	let fixture = Fixture::new(prefix, AssetMode::Production);
	let probe = NavigationProbe::new(StatusCode::NOT_FOUND);
	// Act
	let response = fixture
		.middleware
		.process(
			Request::builder()
				.uri("/console/nested/route")
				.header(header::ACCEPT, "text/html")
				.build()
				.unwrap(),
			probe,
		)
		.await
		.unwrap();
	let html = std::str::from_utf8(&response.body).unwrap();
	// Assert
	assert_eq!(response.status, StatusCode::OK);
	assert_eq!(response.headers[header::CACHE_CONTROL], "no-cache");
	assert!(html.contains(&fixture.url("logo.svg")), "{html}");
	assert!(html.contains(&fixture.url("app.js")), "{html}");
	assert!(html.contains(&fixture.url("app_bg.wasm")), "{html}");
	assert_eq!(html.matches("<link ").count(), 1);
	assert_eq!(html.matches("id=\"reinhardt-static-assets\"").count(), 1);
	assert!(
		html.find("id=\"reinhardt-static-assets\"").unwrap() < html.find("const image").unwrap()
	);
	assert!(html.contains("module_or_path: entry.wasm"));
	assert!(!html.contains("navigation-probe"));
}

#[rstest]
#[case("app.js", "text/javascript")]
#[case("app_bg.wasm", "application/wasm")]
#[case("site.css", "text/css")]
#[case("logo.svg", "image/svg+xml")]
#[tokio::test]
async fn immutable_assets_have_mime_head_and_conditional_response_parity(
	#[case] logical: &str,
	#[case] mime: &str,
) {
	// Arrange
	let fixture = Fixture::new("/console/static/", AssetMode::Production);
	let probe = NavigationProbe::new(StatusCode::OK);
	let uri = fixture.url(logical);
	// Act
	let get = fixture
		.middleware
		.process(Request::builder().uri(&uri).build().unwrap(), probe.clone())
		.await
		.unwrap();
	let head = fixture
		.middleware
		.process(
			Request::builder()
				.uri(&uri)
				.method(Method::HEAD)
				.build()
				.unwrap(),
			probe.clone(),
		)
		.await
		.unwrap();
	let conditional = fixture
		.middleware
		.process(
			Request::builder()
				.uri(&uri)
				.header(header::IF_NONE_MATCH, &get.headers[header::ETAG])
				.build()
				.unwrap(),
			probe.clone(),
		)
		.await
		.unwrap();
	// Assert
	assert_eq!(get.status, StatusCode::OK);
	assert_eq!(get.headers[header::CONTENT_TYPE], mime);
	assert_eq!(
		get.headers[header::CACHE_CONTROL],
		"public, max-age=31536000, immutable"
	);
	assert_eq!(
		head.headers[header::CONTENT_LENGTH],
		get.headers[header::CONTENT_LENGTH]
	);
	assert_eq!(head.headers[header::ETAG], get.headers[header::ETAG]);
	assert!(head.body.is_empty());
	assert_eq!(conditional.status, StatusCode::NOT_MODIFIED);
	assert!(conditional.body.is_empty());
	assert_eq!(probe.call_count(), 0);
}

#[rstest]
#[case("/static/builds/%2e%2e/private")]
#[case("/static/builds/%2Fprivate")]
#[case("/static/builds/%5cprivate")]
#[case("/static/builds/%ZZ")]
#[case("/static/.publication.lock")]
#[case("/static/unknown")]
#[case("/static/missing.js")]
#[tokio::test]
async fn invalid_or_reserved_asset_paths_never_become_successful_html(#[case] uri: &str) {
	// Arrange
	let fixture = Fixture::new("/static/", AssetMode::Production);
	let probe = NavigationProbe::new(StatusCode::OK);
	// Act
	let response = fixture
		.middleware
		.process(Request::builder().uri(uri).build().unwrap(), probe.clone())
		.await
		.unwrap();
	// Assert
	assert!(response.status.is_client_error());
	assert_ne!(response.headers[header::CONTENT_TYPE], "text/html");
	assert_eq!(response.headers[header::CACHE_CONTROL], "no-store");
	assert_eq!(probe.call_count(), 0);
}

fn body_bytes(response: &Response) -> Vec<u8> {
	match response.file_body() {
		Some(body) => body
			.read_chunk(0, usize::try_from(body.len()).unwrap())
			.unwrap()
			.to_vec(),
		None => response.body.to_vec(),
	}
}

#[rstest]
#[case("/static-near/route", true, true, 404, 200)]
#[case("/static-near/route", false, true, 404, 404)]
#[case("/api/account", true, true, 404, 404)]
#[case("/docs/assets.js", true, true, 200, 200)]
#[case("/account", true, true, 200, 200)]
#[case("/account", true, false, 404, 404)]
#[tokio::test]
async fn navigation_preserves_routes_and_explicit_fallback(
	#[case] uri: &str,
	#[case] spa: bool,
	#[case] accepts_html: bool,
	#[case] next_status: u16,
	#[case] expected: u16,
) {
	// Arrange
	let fixture = Fixture::new("/static/", AssetMode::Production);
	let middleware = ManifestStaticMiddleware::new(
		ManifestServingConfig::new(fixture.store, "/static/".into())
			.unwrap()
			.with_navigation_fallback(spa),
	);
	let probe = NavigationProbe::new(StatusCode::from_u16(next_status).unwrap());
	let request = Request::builder()
		.uri(uri)
		.header(
			header::ACCEPT,
			if accepts_html {
				"text/html"
			} else {
				"application/json"
			},
		)
		.build()
		.unwrap();
	// Act
	let response = middleware.process(request, probe.clone()).await.unwrap();
	// Assert
	assert_eq!(response.status.as_u16(), expected);
	assert_eq!(probe.call_count(), 1);
	assert_eq!(
		body_bytes(&response) == b"navigation-probe",
		expected == next_status
	);
}

#[rstest]
#[case("app.js")]
#[case("app_bg.wasm")]
#[case("site.css")]
#[tokio::test]
async fn removed_published_files_fail_even_on_head_and_conditional_requests(#[case] logical: &str) {
	// Arrange
	let fixture = Fixture::new("/static/", AssetMode::Production);
	let uri = fixture.url(logical);
	let snapshot = fixture.store.active();
	std::fs::remove_file(
		fixture
			._root
			.path()
			.join(&snapshot.manifest().paths[logical]),
	)
	.unwrap();
	let probe = NavigationProbe::new(StatusCode::OK);
	// Act / Assert
	for method in [Method::GET, Method::HEAD] {
		let response = fixture
			.middleware
			.process(
				Request::builder()
					.uri(&uri)
					.method(method.clone())
					.header(header::IF_NONE_MATCH, "*")
					.build()
					.unwrap(),
				probe.clone(),
			)
			.await
			.unwrap();
		assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);
		assert_eq!(response.headers[header::CACHE_CONTROL], "no-store");
		assert_eq!(
			response.headers[header::CONTENT_TYPE],
			"text/plain; charset=utf-8"
		);
		if method == Method::HEAD {
			assert!(body_bytes(&response).is_empty());
		}
	}
	assert_eq!(probe.call_count(), 0);
}

#[rstest]
#[case("clip.mp4", "video/mp4", "videos")]
#[case("logo.svg", "image/svg+xml", "vectors")]
#[case("logo.png", "image/png", "images")]
#[case("font.woff2", "font/woff2", "fonts")]
#[case("song.mp3", "audio/mpeg", "audio")]
#[case("payload.data", "application/octet-stream", "other")]
#[tokio::test]
async fn every_category_streams_the_actual_bytes_and_supports_ranges(
	#[case] logical: &str,
	#[case] mime: &str,
	#[case] category: &str,
) {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let bytes = b"0123456789abcdefghijklmnopqrstuvwxyz";
	let mut pipeline = AssetPipeline::new();
	pipeline
		.add_input(AssetInput::bytes(logical, bytes.to_vec()))
		.unwrap();
	let snapshot = AssetPublisher::new(root.path().into())
		.publish(pipeline.prepare(AssetMode::Production).unwrap())
		.unwrap();
	let url = snapshot
		.url_snapshot("/static/")
		.unwrap()
		.resolve(logical)
		.unwrap();
	let store =
		Arc::new(ManifestStore::open(root.path().into(), SnapshotOptions::production()).unwrap());
	let middleware = ManifestStaticMiddleware::new(
		ManifestServingConfig::new(store, "/static/".into()).unwrap(),
	);
	let next = NavigationProbe::new(StatusCode::OK);
	// Act
	let get = middleware
		.process(Request::builder().uri(&url).build().unwrap(), next.clone())
		.await
		.unwrap();
	let range = middleware
		.process(
			Request::builder()
				.uri(&url)
				.header(header::RANGE, "bytes=10-15")
				.build()
				.unwrap(),
			next.clone(),
		)
		.await
		.unwrap();
	let missing = middleware
		.process(
			Request::builder()
				.uri(&url)
				.header(header::RANGE, "bytes=1000-")
				.build()
				.unwrap(),
			next,
		)
		.await
		.unwrap();
	// Assert
	assert!(url.contains(&format!("/{category}/")), "{url}");
	assert_eq!(get.headers[header::CONTENT_TYPE], mime);
	assert_eq!(body_bytes(&get), bytes);
	assert!(get.file_body().is_some());
	assert!(get.body.is_empty());
	assert_eq!(range.status, StatusCode::PARTIAL_CONTENT);
	assert_eq!(range.headers[header::CONTENT_RANGE], "bytes 10-15/36");
	assert_eq!(body_bytes(&range), b"abcdef");
	assert_eq!(missing.status, StatusCode::RANGE_NOT_SATISFIABLE);
	assert_eq!(missing.headers[header::CONTENT_RANGE], "bytes */36");
}

#[rstest]
#[case("gzip", Some("gzip"), 200)]
#[case("br, gzip", Some("br"), 200)]
#[case("gzip;q=0, br;q=0", None, 200)]
#[case("identity;q=0, *;q=0", None, 406)]
#[case("gzip;q=0.5, identity;q=0.1", Some("gzip"), 200)]
#[case("br;Q=0, identity;q=0", None, 406)]
#[case("gzip;Q=0.5, br;q=0.2, identity;Q=0.1", Some("gzip"), 200)]
#[case("gzip;q=0, br;Q=0", None, 200)]
#[tokio::test]
async fn negotiated_representations_have_distinct_etags_and_correct_bytes(
	#[case] accept: &str,
	#[case] encoding: Option<&str>,
	#[case] status: u16,
) {
	use std::io::{Read, Write};
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let mut pipeline = AssetPipeline::new();
	let original = b"body { color: red; }";
	pipeline
		.add_input(AssetInput::bytes("site.css", original.to_vec()))
		.unwrap();
	let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
	gzip.write_all(original).unwrap();
	pipeline
		.add_input(AssetInput::bytes("site.css.gz", gzip.finish().unwrap()))
		.unwrap();
	let mut br = Vec::new();
	brotli::BrotliCompress(
		&mut &original[..],
		&mut br,
		&brotli::enc::BrotliEncoderParams::default(),
	)
	.unwrap();
	pipeline
		.add_input(AssetInput::bytes("site.css.br", br))
		.unwrap();
	let snapshot = AssetPublisher::new(root.path().into())
		.publish(pipeline.prepare(AssetMode::Production).unwrap())
		.unwrap();
	let url = snapshot
		.url_snapshot("/static/")
		.unwrap()
		.resolve("site.css")
		.unwrap();
	let store =
		Arc::new(ManifestStore::open(root.path().into(), SnapshotOptions::production()).unwrap());
	let middleware = ManifestStaticMiddleware::new(
		ManifestServingConfig::new(store, "/static/".into()).unwrap(),
	);
	let next = NavigationProbe::new(StatusCode::OK);
	// Act
	let identity = middleware
		.process(Request::builder().uri(&url).build().unwrap(), next.clone())
		.await
		.unwrap();
	let response = middleware
		.process(
			Request::builder()
				.uri(&url)
				.header(header::ACCEPT_ENCODING, accept)
				.build()
				.unwrap(),
			next,
		)
		.await
		.unwrap();
	// Assert
	assert_eq!(response.status.as_u16(), status);
	assert_eq!(
		response
			.headers
			.get(header::CONTENT_ENCODING)
			.map(|v| v.to_str().unwrap()),
		encoding
	);
	if status == 406 {
		assert_eq!(response.headers[header::CACHE_CONTROL], "no-store");
		return;
	}
	assert_eq!(response.headers[header::VARY], "Accept-Encoding");
	let representation = body_bytes(&response);
	let mut decoded = Vec::new();
	match encoding {
		Some("gzip") => {
			flate2::read::GzDecoder::new(&representation[..])
				.read_to_end(&mut decoded)
				.unwrap();
		}
		Some("br") => {
			brotli::Decompressor::new(&representation[..], 4096)
				.read_to_end(&mut decoded)
				.unwrap();
		}
		_ => decoded = representation,
	}
	assert_eq!(decoded, original);
	assert_eq!(
		response.headers[header::ETAG] == identity.headers[header::ETAG],
		encoding.is_none()
	);
}

#[rstest]
#[case(
	AssetMode::Production,
	"no-cache",
	"public, max-age=31536000, immutable"
)]
#[case(AssetMode::Development, "no-store", "no-store")]
#[tokio::test]
async fn manifest_and_alias_caching_is_explicit(
	#[case] mode: AssetMode,
	#[case] refresh: &str,
	#[case] immutable: &str,
) {
	// Arrange
	let fixture = Fixture::new("/static/", mode);
	let generation = fixture.store.active().manifest().build_id.clone();
	let alias_middleware = ManifestStaticMiddleware::new(
		ManifestServingConfig::new(fixture.store.clone(), "/static/".into())
			.unwrap()
			.with_legacy_aliases(true),
	);
	let next = NavigationProbe::new(StatusCode::OK);
	// Act / Assert
	for (uri, cache) in [
		("/static/manifest.json".to_owned(), refresh),
		(
			format!("/static/builds/{generation}/manifest.json"),
			immutable,
		),
		(fixture.url("site.css"), immutable),
	] {
		let response = fixture
			.middleware
			.process(Request::builder().uri(&uri).build().unwrap(), next.clone())
			.await
			.unwrap();
		assert_eq!(response.status, StatusCode::OK);
		assert_eq!(response.headers[header::CACHE_CONTROL], cache);
	}
	let alias = alias_middleware
		.process(
			Request::builder().uri("/static/site.css").build().unwrap(),
			next.clone(),
		)
		.await
		.unwrap();
	let disabled = fixture
		.middleware
		.process(
			Request::builder().uri("/static/site.css").build().unwrap(),
			next,
		)
		.await
		.unwrap();
	assert_eq!(alias.headers[header::CACHE_CONTROL], refresh);
	assert_eq!(body_bytes(&alias), b"body{color:rgb(1,2,3)}");
	assert_eq!(disabled.status, StatusCode::NOT_FOUND);
}

#[rstest]
#[tokio::test]
async fn retained_document_urls_and_asset_urls_stay_bound_during_activation_and_rollback() {
	// Arrange
	let fixture = Fixture::new("/console/static/", AssetMode::Production);
	let a = fixture.store.active();
	let a_manifest = a.manifest_bytes().to_vec();
	let a_js = fixture.url("app.js");
	let a_doc = fixture.url("index.html");
	let mut b = AssetPipeline::new();
	for (logical, bytes) in [("app.js", &b"export const wasm = new URL('app_bg.wasm',import.meta.url); export default function init(){return 2;}"[..]), ("app_bg.wasm", b"\0asm\x01\0\0\0")] {
		b.add_input(AssetInput::bytes(logical, bytes.to_vec()).with_producer(AssetProducer::Pages)).unwrap();
	}
	b.set_entrypoint(
		"default",
		PagesEntrypoint {
			javascript: "app.js".into(),
			wasm: "app_bg.wasm".into(),
			styles: vec![],
			document: None,
		},
	)
	.unwrap();
	AssetPublisher::new(fixture._root.path().into())
		.publish(b.prepare(AssetMode::Production).unwrap())
		.unwrap();
	fixture.store.reload().unwrap();
	let b_id = fixture.store.active().manifest().build_id.clone();
	let next = NavigationProbe::new(StatusCode::NOT_FOUND);
	// Act
	let old_document = fixture
		.middleware
		.process(
			Request::builder().uri(&a_doc).build().unwrap(),
			next.clone(),
		)
		.await
		.unwrap();
	let new_document = fixture
		.middleware
		.process(
			Request::builder()
				.uri("/screen")
				.header(header::ACCEPT, "text/html")
				.build()
				.unwrap(),
			next.clone(),
		)
		.await
		.unwrap();
	let old_js = fixture
		.middleware
		.process(Request::builder().uri(&a_js).build().unwrap(), next.clone())
		.await
		.unwrap();
	std::fs::write(fixture._root.path().join("manifest.json"), b"incomplete").unwrap();
	let reload_error = fixture.store.reload().unwrap_err();
	let after_failed_reload = fixture.store.active();
	std::fs::write(fixture._root.path().join("manifest.json"), a_manifest).unwrap();
	fixture.store.reload().unwrap();
	let rollback = fixture
		.middleware
		.process(
			Request::builder()
				.uri("/screen")
				.header(header::ACCEPT, "text/html")
				.build()
				.unwrap(),
			next,
		)
		.await
		.unwrap();
	// Assert
	let text = |response: &Response| String::from_utf8(body_bytes(response)).unwrap();
	assert!(text(&old_document).contains(&a_js));
	assert!(!text(&old_document).contains(&b_id));
	assert!(text(&new_document).contains(&b_id));
	assert!(!text(&new_document).contains(&a.manifest().build_id));
	assert_eq!(old_document.headers[header::CACHE_CONTROL], "no-cache");
	assert_eq!(body_bytes(&old_js), a.read_asset("app.js").unwrap());
	assert!(reload_error.to_string().contains("manifest"));
	assert_eq!(after_failed_reload.manifest().build_id, b_id);
	assert!(text(&rollback).contains(&a_js));
	assert_ne!(
		new_document.headers[header::ETAG],
		old_document.headers[header::ETAG]
	);
	assert_eq!(
		rollback.headers[header::ETAG],
		old_document.headers[header::ETAG]
	);
}

#[rstest]
#[tokio::test]
async fn encoded_unicode_and_percent_filenames_are_decoded_once() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let mut pipeline = AssetPipeline::new();
	let name = "images/日本語%20.svg";
	pipeline
		.add_input(AssetInput::bytes(name, b"<svg/>".to_vec()))
		.unwrap();
	let snapshot = AssetPublisher::new(root.path().into())
		.publish(pipeline.prepare(AssetMode::Production).unwrap())
		.unwrap();
	let prefix = "/%E7%94%BB%E9%9D%A2/static/";
	let url = snapshot
		.url_snapshot(prefix)
		.unwrap()
		.resolve(name)
		.unwrap();
	let store =
		Arc::new(ManifestStore::open(root.path().into(), SnapshotOptions::production()).unwrap());
	let middleware =
		ManifestStaticMiddleware::new(ManifestServingConfig::new(store, prefix.into()).unwrap());
	// Act
	let response = middleware
		.process(
			Request::builder().uri(&url).build().unwrap(),
			NavigationProbe::new(StatusCode::OK),
		)
		.await
		.unwrap();
	// Assert
	assert_eq!(response.status, StatusCode::OK);
	assert_eq!(body_bytes(&response), b"<svg/>");
	assert!(url.contains("%2520.svg"));
}

#[rstest]
#[case("/docs/static/", "/docs/guide")]
#[case("/api/assets/", "/api/items")]
#[tokio::test]
async fn static_mount_precedes_overlapping_passthrough(
	#[case] prefix: &str,
	#[case] application: &str,
) {
	// Arrange
	let fixture = Fixture::new(prefix, AssetMode::Production);
	let probe = NavigationProbe::new(StatusCode::OK);
	// Act
	let asset = fixture
		.middleware
		.process(
			Request::builder()
				.uri(fixture.url("site.css"))
				.build()
				.unwrap(),
			probe.clone(),
		)
		.await
		.unwrap();
	let manifest = fixture
		.middleware
		.process(
			Request::builder()
				.uri(format!("{prefix}manifest.json"))
				.build()
				.unwrap(),
			probe.clone(),
		)
		.await
		.unwrap();
	let missing = fixture
		.middleware
		.process(
			Request::builder()
				.uri(format!("{prefix}missing.js"))
				.build()
				.unwrap(),
			probe.clone(),
		)
		.await
		.unwrap();
	// Assert
	assert_eq!(asset.status, StatusCode::OK);
	assert_eq!(
		asset
			.file_body()
			.unwrap()
			.read_chunk(0, 1024)
			.unwrap()
			.as_ref(),
		b"body{color:rgb(1,2,3)}"
	);
	assert_eq!(manifest.status, StatusCode::OK);
	assert_eq!(missing.status, StatusCode::NOT_FOUND);
	assert_eq!(probe.call_count(), 0);
	let routed = fixture
		.middleware
		.process(
			Request::builder().uri(application).build().unwrap(),
			probe.clone(),
		)
		.await
		.unwrap();
	assert_eq!(routed.body.as_ref(), b"navigation-probe");
	assert_eq!(probe.call_count(), 1);
}

#[rstest]
#[case(vec![], true)]
#[case(vec!["base.css"], true)]
#[case(vec!["base.css", "overrides.css"], true)]
#[case(vec!["overrides.css"], false)]
#[case(vec!["overrides.css", "base.css"], false)]
fn template_styles_preserve_declared_cascade(#[case] links: Vec<&str>, #[case] valid: bool) {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let mut pipeline = AssetPipeline::new();
	for (name, bytes) in [
		(
			"app.js",
			&b"export const wasm=new URL('app.wasm',import.meta.url);"[..],
		),
		("app.wasm", b"\0asm\x01\0\0\0"),
		("base.css", b"body{color:red}"),
		("overrides.css", b"body{color:blue}"),
	] {
		pipeline
			.add_input(AssetInput::bytes(name, bytes.to_vec()).with_producer(
				if name.ends_with(".js") || name.ends_with(".wasm") {
					AssetProducer::Pages
				} else {
					AssetProducer::Static
				},
			))
			.unwrap();
	}
	let tags = links
		.iter()
		.map(|name| format!("<link rel=\"stylesheet\" href=\"{name}\">"))
		.collect::<String>();
	pipeline
		.add_input(
			AssetInput::bytes(
				"index.html",
				format!("<html><head>{tags}</head><body></body></html>").into_bytes(),
			)
			.with_role(AssetRole::EntryDocument),
		)
		.unwrap();
	pipeline
		.set_entrypoint(
			"app",
			PagesEntrypoint {
				javascript: "app.js".into(),
				wasm: "app.wasm".into(),
				styles: vec!["base.css".into(), "overrides.css".into()],
				document: Some("index.html".into()),
			},
		)
		.unwrap();
	// Act
	let result = pipeline.prepare(AssetMode::Production);
	// Assert
	if valid {
		let snapshot = AssetPublisher::new(root.path().into())
			.publish(result.unwrap())
			.unwrap();
		let template = String::from_utf8(snapshot.read_asset("index.html").unwrap()).unwrap();
		let html = reinhardt_utils::staticfiles::publication::render_entry_document(
			&snapshot, "/static/", "app", &template,
		)
		.unwrap();
		let projection = snapshot.url_snapshot("/static/").unwrap();
		let base = projection.resolve("base.css").unwrap();
		let overrides = projection.resolve("overrides.css").unwrap();
		let base_link = format!("<link rel=\"stylesheet\" href=\"{base}\">");
		let override_link = format!("<link rel=\"stylesheet\" href=\"{overrides}\">");
		assert_eq!(html.matches(&base_link).count(), 1);
		assert_eq!(html.matches(&override_link).count(), 1);
		assert!(html.find(&base_link).unwrap() < html.find(&override_link).unwrap());
	} else {
		assert_eq!(
			result.unwrap_err().to_string(),
			"invalid static asset manifest: entrypoint \"app\" template stylesheet links must form a prefix of its declared cascade order before the styles slot; include all styles in order or leave them for injection"
		);
	}
}

#[rstest]
#[case("text/html;q=0", 404)]
#[case("text/html;q=0.000, */*;q=1", 404)]
#[case("application/json, text/html; q=0", 404)]
#[case("text/html;q=invalid", 404)]
#[case("text/html;q=1.1", 404)]
#[case("text/html;q=0.5", 200)]
#[case("TEXT/HTML; Q=1", 200)]
#[case("text/html; charset=utf-8", 200)]
#[tokio::test]
async fn navigation_honors_html_quality(#[case] accept: &str, #[case] status: u16) {
	// Arrange
	let fixture = Fixture::new("/static/", AssetMode::Production);
	let probe = NavigationProbe::new(StatusCode::NOT_FOUND);
	// Act
	let response = fixture
		.middleware
		.process(
			Request::builder()
				.uri("/screen")
				.header(header::ACCEPT, accept)
				.build()
				.unwrap(),
			probe.clone(),
		)
		.await
		.unwrap();
	// Assert
	assert_eq!(response.status.as_u16(), status);
	assert_eq!(probe.call_count(), 1);
	if status == 404 {
		assert_eq!(response.body.as_ref(), b"navigation-probe");
	}
}

#[rstest]
#[case("bytes=0-1,4-5")]
#[case("bytes=0-1,999-1000")]
#[case("items=0-1")]
#[tokio::test]
async fn unsupported_ranges_serve_the_full_asset(#[case] range: &str) {
	// Arrange
	let fixture = Fixture::new("/static/", AssetMode::Production);
	let probe = NavigationProbe::new(StatusCode::OK);
	// Act
	let response = fixture
		.middleware
		.process(
			Request::builder()
				.uri(fixture.url("site.css"))
				.header(header::RANGE, range)
				.build()
				.unwrap(),
			probe.clone(),
		)
		.await
		.unwrap();
	// Assert
	assert_eq!(response.status, StatusCode::OK);
	assert_eq!(body_bytes(&response), b"body{color:rgb(1,2,3)}");
	assert_eq!(response.headers[header::CONTENT_LENGTH], "22");
	assert!(!response.headers.contains_key(header::CONTENT_RANGE));
	assert_eq!(probe.call_count(), 0);
}

#[rstest]
#[case("/static/")]
#[case("/console/assets/")]
#[case("/assets%20v2/")]
#[case("/assets%2520v2/")]
#[case("/")]
#[tokio::test]
async fn explicit_nested_admin_mount_reaches_router(#[case] prefix: &str) {
	// Arrange
	let fixture = Fixture::new(prefix, AssetMode::Production);
	let middleware = ManifestStaticMiddleware::new(
		ManifestServingConfig::new(fixture.store.clone(), prefix.into())
			.unwrap()
			.with_passthrough_prefixes(vec![format!("{prefix}admin")])
			.unwrap(),
	);
	let probe = NavigationProbe::new(StatusCode::OK);
	// Act
	for name in [
		"admin/style.css",
		"admin/main.js",
		"admin/vendor/open-props.min.css",
	] {
		let response = middleware
			.process(
				Request::builder()
					.uri(format!("{prefix}{name}"))
					.build()
					.unwrap(),
				probe.clone(),
			)
			.await
			.unwrap();
		// Assert
		assert_eq!(response.body.as_ref(), b"navigation-probe");
	}
	assert_eq!(probe.call_count(), 3);
	for name in [
		"administrator/style.css",
		"missing.js",
		"builds/missing/admin/style.css",
	] {
		let response = middleware
			.process(
				Request::builder()
					.uri(format!("{prefix}{name}"))
					.build()
					.unwrap(),
				probe.clone(),
			)
			.await
			.unwrap();
		assert_eq!(response.status, StatusCode::NOT_FOUND);
	}
	assert_eq!(probe.call_count(), 3);
	let asset = middleware
		.process(
			Request::builder()
				.uri(fixture.url("site.css"))
				.build()
				.unwrap(),
			probe,
		)
		.await
		.unwrap();
	assert_eq!(body_bytes(&asset), b"body{color:rgb(1,2,3)}");
}
