//! Custom protocol handler for serving bundled assets.

use std::borrow::Cow;
use std::collections::HashMap;

use crate::error::{DesktopError, Result};

/// The custom protocol scheme used for serving assets.
pub(crate) const PROTOCOL_SCHEME: &str = "reinhardt";

/// Handles custom protocol requests for serving bundled assets.
#[derive(Debug, Default)]
pub struct ProtocolHandler {
	/// Embedded assets (path -> content).
	assets: HashMap<String, Asset>,
}

/// An embedded asset with content and MIME type.
#[derive(Debug, Clone)]
pub struct Asset {
	/// The asset content.
	pub content: Cow<'static, [u8]>,
	/// The MIME type.
	pub mime_type: String,
}

impl Asset {
	/// Creates a new asset with the given content and MIME type.
	pub fn new(content: impl Into<Cow<'static, [u8]>>, mime_type: impl Into<String>) -> Self {
		Self {
			content: content.into(),
			mime_type: mime_type.into(),
		}
	}

	/// Creates an HTML asset.
	pub fn html(content: impl Into<Cow<'static, [u8]>>) -> Self {
		Self::new(content, "text/html")
	}

	/// Creates a CSS asset.
	pub fn css(content: impl Into<Cow<'static, [u8]>>) -> Self {
		Self::new(content, "text/css")
	}

	/// Creates a JavaScript asset.
	pub fn js(content: impl Into<Cow<'static, [u8]>>) -> Self {
		Self::new(content, "application/javascript")
	}

	/// Creates a JSON asset.
	pub fn json(content: impl Into<Cow<'static, [u8]>>) -> Self {
		Self::new(content, "application/json")
	}
}

impl ProtocolHandler {
	/// Creates a new protocol handler.
	pub fn new() -> Self {
		Self::default()
	}

	/// Registers an asset at the given path.
	pub fn register_asset(&mut self, path: impl Into<String>, asset: Asset) {
		self.assets.insert(path.into(), asset);
	}

	/// Registers an HTML string at the given path.
	pub fn register_html(&mut self, path: impl Into<String>, content: impl Into<String>) {
		let content_string = content.into();
		self.register_asset(path, Asset::html(content_string.into_bytes()));
	}

	/// Resolves a path to its asset content.
	pub fn resolve(&self, path: &str) -> Result<&Asset> {
		// Normalize path (remove leading slash if present)
		let normalized = path.trim_start_matches('/');

		self.assets
			.get(normalized)
			.or_else(|| self.assets.get(path))
			.ok_or_else(|| DesktopError::AssetNotFound(path.to_string()))
	}

	/// Returns the full URL for a path using the custom protocol.
	pub fn url_for(path: &str) -> String {
		format!(
			"{}://localhost/{}",
			PROTOCOL_SCHEME,
			path.trim_start_matches('/')
		)
	}

	/// Infers MIME type from file extension.
	pub fn mime_type_for_extension(ext: &str) -> &'static str {
		match ext.to_lowercase().as_str() {
			"html" | "htm" => "text/html",
			"css" => "text/css",
			"js" | "mjs" => "application/javascript",
			"json" => "application/json",
			"png" => "image/png",
			"jpg" | "jpeg" => "image/jpeg",
			"gif" => "image/gif",
			"svg" => "image/svg+xml",
			"ico" => "image/x-icon",
			"woff" => "font/woff",
			"woff2" => "font/woff2",
			"ttf" => "font/ttf",
			"otf" => "font/otf",
			"wasm" => "application/wasm",
			_ => "application/octet-stream",
		}
	}
}
