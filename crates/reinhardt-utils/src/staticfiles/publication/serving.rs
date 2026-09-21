//! Exact manifest mounts and generation-pinned document responses.

use super::{
	AssetBuildError, AssetEncoding, AssetMode, AssetRole, ManifestSnapshot, ManifestStore,
	render_entry_document,
};
use async_trait::async_trait;
use http::{HeaderMap, Method, StatusCode, header};
use reinhardt_http::{Handler, Middleware, Request, Response};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Validated URL mount over one complete publication store (P0).
#[derive(Debug, Clone)]
pub struct ManifestServingConfig {
	store: Arc<ManifestStore>,
	static_url: String,
	prefix: String,
	entrypoint: Option<String>,
	navigation_fallback: bool,
	legacy_aliases: bool,
	passthrough: Vec<String>,
}

impl ManifestServingConfig {
	/// Normalize the public URL and derive its path mount, including CDN origins.
	pub fn new(store: Arc<ManifestStore>, static_url: String) -> Result<Self, AssetBuildError> {
		let active = store.active();
		let projection = active.url_snapshot(&static_url)?;
		let static_url = projection.static_url().to_owned();
		let uri: http::Uri = static_url.parse().map_err(|e: http::uri::InvalidUri| {
			AssetBuildError::input(&static_url, e.to_string())
		})?;
		let prefix = decode_path(uri.path())
			.map_err(|reason| AssetBuildError::input(&static_url, reason))?;
		let entrypoint = (active.manifest().entrypoints.len() == 1).then(|| {
			active
				.manifest()
				.entrypoints
				.keys()
				.next()
				.expect("one entry")
				.clone()
		});
		Ok(Self {
			store,
			static_url,
			prefix,
			entrypoint,
			navigation_fallback: false,
			legacy_aliases: false,
			passthrough: vec!["/api".into(), "/docs".into(), "/openapi.json".into()],
		})
	}
	/// Select a named Pages entry. Call `validate` before starting a server.
	pub fn with_pages(mut self, entrypoint: String) -> Self {
		self.entrypoint = Some(entrypoint);
		self
	}
	/// Enable navigation fallback only after the application router returns 404.
	pub fn with_navigation_fallback(mut self, enabled: bool) -> Self {
		self.navigation_fallback = enabled;
		self
	}
	/// Explicitly serve mutable logical-name aliases with refreshable caching.
	pub fn with_legacy_aliases(mut self, enabled: bool) -> Self {
		self.legacy_aliases = enabled;
		self
	}
	/// Preserve framework/application routes at segment boundaries.
	/// Explicit prefixes nested inside the static mount take precedence over assets.
	pub fn with_passthrough_prefixes(
		mut self,
		prefixes: Vec<String>,
	) -> Result<Self, AssetBuildError> {
		self.passthrough = prefixes
			.into_iter()
			.map(|prefix| {
				decode_path(&prefix).map_err(|reason| AssetBuildError::input(&prefix, reason))
			})
			.collect::<Result<_, _>>()?;
		Ok(self)
	}
	/// Reject unknown or ambiguous entrypoint selection before accepting requests.
	pub fn validate(&self) -> Result<(), AssetBuildError> {
		let active = self.store.active();
		if let Some(entry) = &self.entrypoint {
			if !active.manifest().entrypoints.contains_key(entry) {
				return Err(AssetBuildError::input(
					entry,
					"unknown Pages entrypoint; select a manifest entrypoint name",
				));
			}
		} else if self.navigation_fallback || active.manifest().entrypoints.len() > 1 {
			return Err(AssetBuildError::manifest(
				"select a Pages entrypoint explicitly for navigation or a multi-entry publication",
			));
		}
		Ok(())
	}
}

/// Serves only registered generation files, never arbitrary static-root contents (P0).
#[derive(Debug, Clone)]
pub struct ManifestStaticMiddleware {
	config: ManifestServingConfig,
}

impl ManifestStaticMiddleware {
	/// Create the shared HTTP adapter used by application and standalone servers.
	pub fn new(config: ManifestServingConfig) -> Self {
		Self { config }
	}
}

#[derive(Clone)]
struct RequestHeaders {
	method: Method,
	headers: HeaderMap,
}

enum Target {
	Manifest,
	Asset(String),
	Document(String),
}

#[async_trait]
impl Middleware for ManifestStaticMiddleware {
	async fn process(
		&self,
		request: Request,
		next: Arc<dyn Handler>,
	) -> reinhardt_core::exception::Result<Response> {
		let path = match decode_path(request.uri.path()) {
			Ok(path) => path,
			Err(_) => return Ok(error(StatusCode::BAD_REQUEST, &request.method)),
		};
		// Explicit nested mounts (for example admin assets) belong to the router.
		// An ancestor passthrough such as /docs must not shadow /docs/static/.
		if self
			.config
			.passthrough
			.iter()
			.any(|prefix| prefix.starts_with(&self.config.prefix) && matches_prefix(&path, prefix))
		{
			return next.handle(request).await;
		}
		let active = self.config.store.active();
		let relative = path
			.strip_prefix(&self.config.prefix)
			.or_else(|| (path == self.config.prefix.trim_end_matches('/')).then_some(""));
		let request_headers = RequestHeaders {
			method: request.method.clone(),
			headers: request.headers.clone(),
		};
		if let Some(relative) = relative {
			let selected = if relative == "manifest.json" {
				Some((active.clone(), Target::Manifest, false))
			} else if let Some(generation_path) = relative.strip_prefix("builds/") {
				let Some((id, file)) = generation_path.split_once('/') else {
					return Ok(error(StatusCode::NOT_FOUND, &request.method));
				};
				let Some(snapshot) = self.config.store.generation(id) else {
					return Ok(error(StatusCode::NOT_FOUND, &request.method));
				};
				let target = if file == "manifest.json" {
					Target::Manifest
				} else if let Some(logical) = snapshot.logical_path(file) {
					Target::Asset(logical.into())
				} else {
					return Ok(error(StatusCode::NOT_FOUND, &request.method));
				};
				Some((snapshot, target, true))
			} else if self.config.legacy_aliases && active.manifest().paths.contains_key(relative) {
				Some((active.clone(), Target::Asset(relative.into()), false))
			} else {
				None
			};
			if let Some((snapshot, target, immutable)) = selected {
				return Ok(dispatch(
					self.config.clone(),
					snapshot,
					target,
					immutable,
					request_headers,
				)
				.await);
			}
			if self.config.prefix != "/" || reserved(relative) {
				return Ok(error(StatusCode::NOT_FOUND, &request.method));
			}
		}
		if self
			.config
			.passthrough
			.iter()
			.any(|prefix| matches_prefix(&path, prefix))
		{
			return next.handle(request).await;
		}
		if path
			.rsplit('/')
			.next()
			.and_then(|name| name.rsplit_once('.'))
			.is_some_and(|(_, extension)| {
				matches!(
					extension.to_ascii_lowercase().as_str(),
					"js" | "mjs" | "cjs" | "wasm" | "css"
				)
			}) {
			return Ok(error(StatusCode::NOT_FOUND, &request.method));
		}
		let navigation = matches!(request.method, Method::GET | Method::HEAD)
			&& request
				.headers
				.get(header::ACCEPT)
				.and_then(|value| value.to_str().ok())
				.is_some_and(|value| {
					value.split(',').any(|part| {
						let mut parameters = part.trim().split(';');
						if !parameters
							.next()
							.is_some_and(|media| media.trim().eq_ignore_ascii_case("text/html"))
						{
							return false;
						}
						let quality = parameters
							.filter_map(|parameter| parameter.trim().split_once('='))
							.find(|(name, _)| name.trim().eq_ignore_ascii_case("q"))
							.map_or(Some(1.0), |(_, value)| value.trim().parse::<f32>().ok());
						quality.is_some_and(|quality| quality > 0.0 && quality <= 1.0)
					})
				});
		let response = next.handle(request).await?;
		if response.status == StatusCode::NOT_FOUND && self.config.navigation_fallback && navigation
		{
			let Some(entry) = &self.config.entrypoint else {
				return Ok(error(
					StatusCode::SERVICE_UNAVAILABLE,
					&request_headers.method,
				));
			};
			return Ok(dispatch(
				self.config.clone(),
				active,
				Target::Document(entry.clone()),
				false,
				request_headers,
			)
			.await);
		}
		Ok(response)
	}
}

fn reserved(relative: &str) -> bool {
	relative == "builds"
		|| relative == "staticfiles.json"
		|| relative.split('/').any(|part| part.starts_with('.'))
		|| [
			"pages", "videos", "vectors", "images", "css", "js", "fonts", "audio", "other",
		]
		.contains(&relative.split('/').next().unwrap_or_default())
}

fn matches_prefix(path: &str, prefix: &str) -> bool {
	let prefix = prefix.trim_end_matches('/');
	path == prefix
		|| path
			.strip_prefix(prefix)
			.is_some_and(|tail| tail.starts_with('/'))
}

// Strict, exactly-once decoding is shared by request paths and the configured mount.
fn decode_path(path: &str) -> Result<String, &'static str> {
	if !path.starts_with('/') {
		return Err("mount must be an absolute URL path");
	}
	let mut decoded = String::new();
	for (index, part) in path.split('/').enumerate() {
		if index > 0 {
			decoded.push('/');
		}
		let mut bytes = Vec::new();
		let mut remaining = part.as_bytes();
		while let Some((&byte, rest)) = remaining.split_first() {
			if byte == b'%' {
				if rest.len() < 2 {
					return Err("incomplete percent escape");
				}
				let digit = |byte: u8| (byte as char).to_digit(16).map(|value| value as u8);
				let (Some(high), Some(low)) = (digit(rest[0]), digit(rest[1])) else {
					return Err("invalid percent escape");
				};
				bytes.push(high * 16 + low);
				remaining = &rest[2..];
			} else {
				bytes.push(byte);
				remaining = rest;
			}
		}
		let part = std::str::from_utf8(&bytes).map_err(|_| "non-UTF-8 URL segment")?;
		if matches!(part, "." | "..")
			|| part.contains(['/', '\\'])
			|| part.chars().any(char::is_control)
		{
			return Err("unsafe URL segment");
		}
		decoded.push_str(part);
	}
	Ok(decoded)
}

async fn dispatch(
	config: ManifestServingConfig,
	snapshot: Arc<ManifestSnapshot>,
	target: Target,
	immutable: bool,
	request: RequestHeaders,
) -> Response {
	if !matches!(request.method, Method::GET | Method::HEAD) {
		return error(StatusCode::METHOD_NOT_ALLOWED, &request.method)
			.with_header("allow", "GET, HEAD");
	}
	let method = request.method.clone();
	match tokio::task::spawn_blocking(move || {
		serve(&config, &snapshot, target, immutable, &request)
	})
	.await
	{
		Ok(Ok(response)) => response,
		Ok(Err(failure)) => {
			tracing::error!(error = %failure, "static publication is unavailable; verify or republish the complete generation");
			error(StatusCode::SERVICE_UNAVAILABLE, &method)
		}
		Err(failure) => {
			tracing::error!(error = %failure, "static response worker failed");
			error(StatusCode::SERVICE_UNAVAILABLE, &method)
		}
	}
}

fn serve(
	config: &ManifestServingConfig,
	snapshot: &ManifestSnapshot,
	target: Target,
	immutable: bool,
	request: &RequestHeaders,
) -> Result<Response, AssetBuildError> {
	let cache = if snapshot.manifest().mode == AssetMode::Development {
		"no-store"
	} else if immutable {
		"public, max-age=31536000, immutable"
	} else {
		"no-cache"
	};
	match target {
		Target::Manifest => Ok(buffered(
			snapshot.manifest_bytes().to_vec(),
			"application/json",
			cache,
			request,
		)),
		Target::Document(entry) => serve_document(config, snapshot, &entry, request),
		Target::Asset(logical) => {
			let record = &snapshot.manifest().assets[&logical];
			if record.role == AssetRole::EntryDocument {
				let canonical = record.parent.as_deref().unwrap_or(&logical);
				let entry = config
					.entrypoint
					.as_ref()
					.filter(|entry| {
						snapshot
							.manifest()
							.entrypoints
							.get(*entry)
							.is_some_and(|entry| entry.document.as_deref() == Some(canonical))
					})
					.or_else(|| {
						snapshot
							.manifest()
							.entrypoints
							.iter()
							.find(|(_, entry)| entry.document.as_deref() == Some(canonical))
							.map(|(name, _)| name)
					});
				return match entry {
					Some(entry) => serve_document(config, snapshot, entry, request),
					None => Ok(error(StatusCode::NOT_FOUND, &request.method)),
				};
			}
			let Some(selected) = representation(snapshot, &logical, &request.headers) else {
				return Ok(error(StatusCode::NOT_ACCEPTABLE, &request.method));
			};
			let record = &snapshot.manifest().assets[selected];
			// Revalidate even HEAD/304: missing or corrupt deployed files cannot be hidden by validators.
			let file = snapshot.open_asset(selected)?;
			let etag = format!("\"{}\"", record.sha256);
			let mut response = Response::ok()
				.with_header("content-type", &record.mime)
				.with_header("cache-control", cache)
				.with_header("etag", &etag)
				.with_header("accept-ranges", "bytes")
				.with_header("x-content-type-options", "nosniff");
			if !snapshot.manifest().assets[&logical].variants.is_empty() {
				response = response.with_header("vary", "Accept-Encoding");
			}
			if let Some(encoding) = record.encoding {
				response = response.with_header("content-encoding", encoding_name(encoding));
			}
			if not_modified(&request.headers, &etag) {
				response.status = StatusCode::NOT_MODIFIED;
				return Ok(response);
			}
			let range = if request.method == Method::GET
				&& request
					.headers
					.get(header::IF_RANGE)
					.is_none_or(|value| value.as_bytes() == etag.as_bytes())
			{
				request
					.headers
					.get(header::RANGE)
					.and_then(|value| value.to_str().ok())
					// Multipart ranges and unknown units are unsupported; serve the full representation.
					.filter(|value| value.starts_with("bytes=") && !value.contains(','))
			} else {
				None
			};
			let (offset, length) = if let Some(range) = range {
				match byte_range(range, record.size) {
					Some((offset, length)) => {
						response.status = StatusCode::PARTIAL_CONTENT;
						response = response.with_header(
							"content-range",
							&format!(
								"bytes {offset}-{}/{size}",
								offset + length - 1,
								size = record.size
							),
						);
						(offset, length)
					}
					None => {
						return Ok(error(StatusCode::RANGE_NOT_SATISFIABLE, &request.method)
							.with_header("content-range", &format!("bytes */{}", record.size)));
					}
				}
			} else {
				(0, record.size)
			};
			response = response.with_header("content-length", &length.to_string());
			if request.method == Method::HEAD {
				return Ok(response);
			}
			response
				.with_file_body(file, offset, length)
				.map_err(|e| AssetBuildError::io(selected, e))
		}
	}
}

fn serve_document(
	config: &ManifestServingConfig,
	snapshot: &ManifestSnapshot,
	entry: &str,
	request: &RequestHeaders,
) -> Result<Response, AssetBuildError> {
	let entrypoint = snapshot.manifest().entrypoints.get(entry).ok_or_else(|| {
		AssetBuildError::input(entry, "entrypoint is absent from requested generation")
	})?;
	let template = entrypoint
		.document
		.as_ref()
		.map(|name| snapshot.read_asset(name))
		.transpose()?
		.unwrap_or_default();
	let template =
		std::str::from_utf8(&template).map_err(|e| AssetBuildError::input(entry, e.to_string()))?;
	let html = render_entry_document(snapshot, &config.static_url, entry, template)?;
	let cache = if snapshot.manifest().mode == AssetMode::Development {
		"no-store"
	} else {
		"no-cache"
	};
	Ok(buffered(
		html.into_bytes(),
		"text/html; charset=utf-8",
		cache,
		request,
	))
}

fn buffered(bytes: Vec<u8>, mime: &str, cache: &str, request: &RequestHeaders) -> Response {
	let etag = format!("\"{}\"", hex::encode(Sha256::digest(&bytes)));
	let mut response = Response::ok()
		.with_header("content-type", mime)
		.with_header("cache-control", cache)
		.with_header("etag", &etag)
		.with_header("x-content-type-options", "nosniff");
	if not_modified(&request.headers, &etag) {
		response.status = StatusCode::NOT_MODIFIED;
		return response;
	}
	response = response.with_header("content-length", &bytes.len().to_string());
	if request.method == Method::HEAD {
		response
	} else {
		response.with_body(bytes)
	}
}

fn not_modified(headers: &HeaderMap, etag: &str) -> bool {
	headers
		.get_all(header::IF_NONE_MATCH)
		.iter()
		.filter_map(|value| value.to_str().ok())
		.flat_map(|value| value.split(','))
		.any(|tag| {
			let tag = tag.trim();
			tag == "*" || tag.strip_prefix("W/").unwrap_or(tag) == etag
		})
}

fn encoding_name(encoding: AssetEncoding) -> &'static str {
	match encoding {
		AssetEncoding::Gzip => "gzip",
		AssetEncoding::Brotli => "br",
	}
}

fn representation<'a>(
	snapshot: &'a ManifestSnapshot,
	logical: &'a str,
	headers: &HeaderMap,
) -> Option<&'a str> {
	let record = &snapshot.manifest().assets[logical];
	let preferences: Vec<_> = headers
		.get_all(header::ACCEPT_ENCODING)
		.iter()
		.filter_map(|value| value.to_str().ok())
		.flat_map(|value| value.split(','))
		.map(|part| {
			let mut fields = part.trim().split(';');
			let name = fields
				.next()
				.unwrap_or_default()
				.trim()
				.to_ascii_lowercase();
			let quality = fields
				.find_map(|field| {
					let (name, value) = field.trim().split_once('=')?;
					name.trim()
						.eq_ignore_ascii_case("q")
						.then_some(value.trim())
				})
				.map_or(1.0, |value| {
					value
						.parse::<f32>()
						.ok()
						.filter(|q| q.is_finite() && (0.0..=1.0).contains(q))
						.unwrap_or(0.0)
				});
			(name, quality)
		})
		.collect();
	let quality = |name: &str| {
		preferences
			.iter()
			.find(|(key, _)| key == name)
			.map(|(_, q)| *q)
			.or_else(|| {
				preferences
					.iter()
					.find(|(key, _)| key == "*")
					.and_then(|(_, q)| (name != "identity" || *q == 0.0).then_some(*q))
			})
			.unwrap_or(if name == "identity" { 1.0 } else { 0.0 })
	};
	let mut selected = logical;
	let mut best = quality(record.encoding.map(encoding_name).unwrap_or("identity"));
	for name in &record.variants {
		let encoding = snapshot.manifest().assets[name]
			.encoding
			.expect("validated variant");
		let next = quality(encoding_name(encoding));
		if next > best
			|| next == best
				&& (snapshot.manifest().assets[selected].encoding.is_none()
					|| encoding == AssetEncoding::Brotli)
		{
			best = next;
			selected = name;
		}
	}
	(best > 0.0).then_some(selected)
}

fn byte_range(value: &str, size: u64) -> Option<(u64, u64)> {
	let (start, end) = value.strip_prefix("bytes=")?.split_once('-')?;
	if size == 0 {
		return None;
	}
	if start.is_empty() {
		let length = end.parse::<u64>().ok()?.min(size);
		return (length > 0).then_some((size - length, length));
	}
	let start = start.parse::<u64>().ok()?;
	let end = if end.is_empty() {
		size - 1
	} else {
		end.parse::<u64>().ok()?.min(size - 1)
	};
	(start <= end && start < size).then(|| (start, end - start + 1))
}

fn error(status: StatusCode, method: &Method) -> Response {
	let body = status.canonical_reason().unwrap_or("Static asset error");
	let response = Response::new(status)
		.with_header("content-type", "text/plain; charset=utf-8")
		.with_header("cache-control", "no-store")
		.with_header("x-content-type-options", "nosniff")
		.with_header("content-length", &body.len().to_string());
	if *method == Method::HEAD {
		response
	} else {
		response.with_body(body)
	}
}
