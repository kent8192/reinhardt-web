//! Browser proof for a complete manifest-backed Pages publication.
//!
//! This test deliberately uses the same nested static prefix and generated
//! loader that production serving uses. A successful HTTP response is not
//! enough: the module must fetch and instantiate the WASM asset and the
//! generated stylesheet must affect the document.

#![cfg(not(target_arch = "wasm32"))]

use async_trait::async_trait;
use hyper::{HeaderMap, Method, StatusCode, header};
use reinhardt_http::{Handler, Middleware, Request, Response};
use reinhardt_test::fixtures::wasm::e2e_cdp::{CdpBrowser, CdpConfig};
use reinhardt_utils::staticfiles::publication::{
	AssetInput, AssetMode, AssetPipeline, AssetProducer, AssetPublisher, AssetRole,
	ManifestServingConfig, ManifestStaticMiddleware, ManifestStore, PagesEntrypoint,
	SnapshotOptions,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

struct NotFound;

#[async_trait]
impl Handler for NotFound {
	async fn handle(&self, _: Request) -> reinhardt_core::exception::Result<Response> {
		Ok(Response::new(StatusCode::NOT_FOUND))
	}
}

struct BrowserServer {
	_root: TempDir,
	_task: JoinHandle<()>,
	addr: SocketAddr,
}

impl BrowserServer {
	async fn start() -> Self {
		let root = tempfile::tempdir().expect("create publication root");
		let mut pipeline = AssetPipeline::new();
		pipeline
			.add_input(
				AssetInput::bytes(
					"app.js",
					br##"
const wasmTarget = new URL("app_bg.wasm", import.meta.url);
export default async function start({ module_or_path }) {
  const url = module_or_path || wasmTarget.href;
  const response = await fetch(url);
  if (!response.ok) throw new Error(`WASM request failed: ${response.status}`);
  await WebAssembly.instantiate(await response.arrayBuffer(), {});
  document.querySelector("#status").textContent = "wasm-ready";
}
"##
					.to_vec(),
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
			.add_input(AssetInput::bytes(
				"site.css",
				b"#style-probe { color: rgb(1, 2, 3); }".to_vec(),
			))
			.unwrap();
		pipeline
			.add_input(
				AssetInput::bytes(
					"index.html",
					br#"<!doctype html><html><head><title>manifest e2e</title></head><body><div id="status">loading</div><div id="style-probe">style</div></body></html>"#.to_vec(),
				)
				.with_role(AssetRole::EntryDocument),
			)
			.unwrap();
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
			.publish(pipeline.prepare(AssetMode::Production).unwrap())
			.unwrap();

		let store = Arc::new(
			ManifestStore::open(root.path().into(), SnapshotOptions::production()).unwrap(),
		);
		let middleware = Arc::new(ManifestStaticMiddleware::new(
			ManifestServingConfig::new(store, "/console/static/".into())
				.unwrap()
				.with_pages("default".into())
				.with_navigation_fallback(true),
		));
		let listener = TcpListener::bind("0.0.0.0:0")
			.await
			.expect("bind browser server");
		let addr = listener.local_addr().expect("read browser server address");
		let task = tokio::spawn(async move {
			loop {
				let Ok((stream, _)) = listener.accept().await else {
					break;
				};
				let middleware = Arc::clone(&middleware);
				tokio::spawn(async move {
					if let Err(error) = serve_connection(stream, middleware).await {
						eprintln!("manifest browser server error: {error}");
					}
				});
			}
		});
		Self {
			_root: root,
			_task: task,
			addr,
		}
	}

	fn container_url(&self) -> String {
		let host = if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
			"host.docker.internal"
		} else {
			"172.17.0.1"
		};
		format!("http://{host}:{}", self.addr.port())
	}
}

impl Drop for BrowserServer {
	fn drop(&mut self) {
		self._task.abort();
	}
}

async fn serve_connection(
	mut stream: TcpStream,
	middleware: Arc<ManifestStaticMiddleware>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let mut bytes = Vec::with_capacity(4096);
	let mut chunk = [0_u8; 2048];
	while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
		let count = stream.read(&mut chunk).await?;
		if count == 0 {
			return Ok(());
		}
		bytes.extend_from_slice(&chunk[..count]);
		if bytes.len() > 64 * 1024 {
			return Ok(());
		}
	}
	let request = std::str::from_utf8(&bytes)?;
	let mut lines = request.split("\r\n");
	let first = lines.next().ok_or("missing HTTP request line")?;
	let mut first_fields = first.split_whitespace();
	let method = first_fields.next().ok_or("missing HTTP method")?;
	let path = first_fields.next().ok_or("missing HTTP path")?;
	let mut headers = HeaderMap::new();
	for line in lines {
		if line.is_empty() {
			break;
		}
		let Some((name, value)) = line.split_once(':') else {
			continue;
		};
		if let (Ok(name), Ok(value)) = (
			header::HeaderName::from_bytes(name.trim().as_bytes()),
			header::HeaderValue::from_str(value.trim()),
		) {
			headers.append(name, value);
		}
	}
	let request = Request::builder()
		.method(method.parse::<Method>()?)
		.uri(path)
		.headers(headers)
		.build()?;
	let response = middleware
		.process(request, Arc::new(NotFound))
		.await
		.map_err(|error| error.to_string())?;
	let body = if let Some(file) = response.file_body() {
		file.read_chunk(0, usize::try_from(file.len())?)?
	} else {
		response.body.clone()
	};
	let reason = response.status.canonical_reason().unwrap_or_default();
	let mut output = format!(
		"HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
		response.status.as_u16(),
		reason,
		body.len()
	);
	for (name, value) in &response.headers {
		output.push_str(name.as_str());
		output.push_str(": ");
		output.push_str(value.to_str().unwrap_or_default());
		output.push_str("\r\n");
	}
	output.push_str("\r\n");
	stream.write_all(output.as_bytes()).await?;
	stream.write_all(&body).await?;
	stream.shutdown().await?;
	Ok(())
}

#[tokio::test]
async fn browser_loads_manifest_pages_wasm_and_styles() {
	let server = BrowserServer::start().await;
	let browser = CdpBrowser::start_with_retries(CdpConfig::default(), 1)
		.await
		.expect("start isolated Chrome container");
	let page = browser
		.new_page(&format!("{}/console/dashboard", server.container_url()))
		.await
		.expect("navigate to manifest-backed Pages document");
	page.wait_for("#status")
		.await
		.expect("entry document should render");
	let start = std::time::Instant::now();
	loop {
		let status = page.get_text("#status").await.expect("read WASM status");
		if status.as_deref() == Some("wasm-ready") {
			break;
		}
		assert!(
			start.elapsed() < std::time::Duration::from_secs(10),
			"WASM module did not finish loading; status={status:?}"
		);
		tokio::time::sleep(std::time::Duration::from_millis(100)).await;
	}
	let color = page
		.execute_js("getComputedStyle(document.querySelector('#style-probe')).color")
		.await
		.expect("read generated stylesheet result");
	assert_eq!(color.as_str(), Some("rgb(1, 2, 3)"));
	let content_type = page
		.execute_js(
			"performance.getEntriesByType('resource').some(e => e.name.includes('/pages/') && e.name.endsWith('.wasm'))",
		)
		.await
		.expect("inspect browser resource entries");
	assert_eq!(content_type.as_bool(), Some(true));
}
