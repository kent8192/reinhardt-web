//! End-to-end vendor download pipeline test using a mock HTTP server.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use reinhardt_utils::staticfiles::vendor::{
	AppVendorAsset, Verbosity, download_assets, verify_integrity,
};
use rstest::rstest;
use sha2::{Digest, Sha256};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Respond, ResponseTemplate};

const HTMX_BODY: &[u8] = b"// fake htmx body\nconsole.log('htmx');\n";
const ALPINE_BODY: &[u8] = b"/* fake alpine */\n";

fn sha256_hex(bytes: &[u8]) -> String {
	let mut h = Sha256::new();
	h.update(bytes);
	format!("{:x}", h.finalize())
}

struct CountingResponder {
	body: &'static [u8],
	counter: Arc<AtomicUsize>,
}

impl Respond for CountingResponder {
	fn respond(&self, _: &wiremock::Request) -> ResponseTemplate {
		self.counter.fetch_add(1, Ordering::SeqCst);
		ResponseTemplate::new(200).set_body_bytes(self.body)
	}
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn download_then_skip_on_second_call() {
	// Arrange — start mock server with two assets and a counter.
	let server = MockServer::start().await;
	let htmx_count = Arc::new(AtomicUsize::new(0));
	let alpine_count = Arc::new(AtomicUsize::new(0));

	Mock::given(method("GET"))
		.and(path("/htmx.js"))
		.respond_with(CountingResponder {
			body: HTMX_BODY,
			counter: htmx_count.clone(),
		})
		.mount(&server)
		.await;
	Mock::given(method("GET"))
		.and(path("/alpine.js"))
		.respond_with(CountingResponder {
			body: ALPINE_BODY,
			counter: alpine_count.clone(),
		})
		.mount(&server)
		.await;

	let tmp = tempfile::tempdir().expect("tempdir");
	let base = tmp.path();

	// Build asset descriptors. We need 'static URLs — leak intentionally for the test.
	let htmx_url: &'static str = Box::leak(format!("{}/htmx.js", server.uri()).into_boxed_str());
	let alpine_url: &'static str =
		Box::leak(format!("{}/alpine.js", server.uri()).into_boxed_str());
	let htmx_sha: &'static str = Box::leak(sha256_hex(HTMX_BODY).into_boxed_str());

	let assets: Vec<AppVendorAsset> = vec![
		AppVendorAsset {
			app_label: "test",
			url: htmx_url,
			target: "vendor/htmx.js",
			sha256: htmx_sha, // pinned
		},
		AppVendorAsset {
			app_label: "test",
			url: alpine_url,
			target: "vendor/alpine.js",
			sha256: "", // unverified mode
		},
	];

	// Act — first download.
	let r1 = download_assets(base, &assets, Verbosity::Silent).await;
	// Act — second call should skip both files.
	let r2 = download_assets(base, &assets, Verbosity::Silent).await;

	// Assert
	assert!(r1.is_ok(), "first download failed: {:?}", r1);
	assert!(r2.is_ok(), "second download failed: {:?}", r2);

	// Files exist with correct contents.
	let htmx_disk = std::fs::read(base.join("vendor/htmx.js")).expect("htmx file");
	let alpine_disk = std::fs::read(base.join("vendor/alpine.js")).expect("alpine file");
	assert_eq!(htmx_disk.as_slice(), HTMX_BODY);
	assert_eq!(alpine_disk.as_slice(), ALPINE_BODY);

	// Each URL was fetched exactly once (second call skipped).
	assert_eq!(
		htmx_count.load(Ordering::SeqCst),
		1,
		"htmx must be fetched once"
	);
	assert_eq!(
		alpine_count.load(Ordering::SeqCst),
		1,
		"alpine must be fetched once"
	);

	// Pinned SHA verifies cleanly.
	verify_integrity(&base.join("vendor/htmx.js"), htmx_sha).expect("pinned SHA must verify");
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn download_errors_on_sha_mismatch() {
	// Arrange — server returns body B, but we declare SHA of body A.
	let server = MockServer::start().await;
	let request_count = Arc::new(AtomicUsize::new(0));

	Mock::given(method("GET"))
		.and(path("/x.js"))
		.respond_with(CountingResponder {
			counter: request_count.clone(),
			body: b"actual",
		})
		.mount(&server)
		.await;

	let tmp = tempfile::tempdir().expect("tempdir");
	let url: &'static str = Box::leak(format!("{}/x.js", server.uri()).into_boxed_str());
	let wrong_sha = "0000000000000000000000000000000000000000000000000000000000000000";

	let assets = vec![AppVendorAsset {
		app_label: "test",
		url,
		target: "vendor/x.js",
		sha256: wrong_sha,
	}];

	// Act
	let result = download_assets(tmp.path(), &assets, Verbosity::Silent).await;

	// Assert — the function attempts the download, then rejects it before install.
	assert!(result.is_err(), "expected SHA mismatch error");
	assert_eq!(request_count.load(Ordering::SeqCst), 1, "URL fetched once");
	assert!(
		!tmp.path().join("vendor/x.js").exists(),
		"mismatching download must not be installed"
	);
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn download_preserves_existing_file_on_sha_mismatch() {
	// Arrange — an existing local file must not be replaced by a mismatching response.
	let server = MockServer::start().await;
	let request_count = Arc::new(AtomicUsize::new(0));

	Mock::given(method("GET"))
		.and(path("/x.js"))
		.respond_with(CountingResponder {
			counter: request_count.clone(),
			body: b"attacker-controlled",
		})
		.mount(&server)
		.await;

	let tmp = tempfile::tempdir().expect("tempdir");
	let target = tmp.path().join("vendor/x.js");
	std::fs::create_dir_all(target.parent().expect("target parent")).expect("create vendor dir");
	std::fs::write(&target, b"known previous contents").expect("write existing asset");

	let url: &'static str = Box::leak(format!("{}/x.js", server.uri()).into_boxed_str());
	let wrong_sha = "0000000000000000000000000000000000000000000000000000000000000000";
	let assets = vec![AppVendorAsset {
		app_label: "test",
		url,
		target: "vendor/x.js",
		sha256: wrong_sha,
	}];

	// Act
	let result = download_assets(tmp.path(), &assets, Verbosity::Silent).await;

	// Assert
	assert!(result.is_err(), "expected SHA mismatch error");
	assert_eq!(request_count.load(Ordering::SeqCst), 1, "URL fetched once");
	let contents = std::fs::read(&target).expect("existing asset remains readable");
	assert_eq!(contents, b"known previous contents");
}
