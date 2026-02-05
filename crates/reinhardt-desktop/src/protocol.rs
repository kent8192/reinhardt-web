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

	/// Registers bundled CSS at the standard path (`bundle.css`).
	pub fn register_bundled_css(&mut self, content: impl Into<String>) {
		let content_string = content.into();
		self.register_asset("bundle.css", Asset::css(content_string.into_bytes()));
	}

	/// Registers bundled JS at the standard path (`bundle.js`).
	pub fn register_bundled_js(&mut self, content: impl Into<String>) {
		let content_string = content.into();
		self.register_asset("bundle.js", Asset::js(content_string.into_bytes()));
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

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_asset_html_factory() {
		// Arrange
		let content = b"<html></html>";

		// Act
		let asset = Asset::html(content.as_slice());

		// Assert
		assert_eq!(asset.content.as_ref(), content);
		assert_eq!(asset.mime_type, "text/html");
	}

	#[rstest]
	fn test_asset_css_factory() {
		// Arrange
		let content = b"body { color: red; }";

		// Act
		let asset = Asset::css(content.as_slice());

		// Assert
		assert_eq!(asset.content.as_ref(), content);
		assert_eq!(asset.mime_type, "text/css");
	}

	#[rstest]
	fn test_asset_js_factory() {
		// Arrange
		let content = b"console.log('hello');";

		// Act
		let asset = Asset::js(content.as_slice());

		// Assert
		assert_eq!(asset.content.as_ref(), content);
		assert_eq!(asset.mime_type, "application/javascript");
	}

	#[rstest]
	fn test_asset_json_factory() {
		// Arrange
		let content = b"{}";

		// Act
		let asset = Asset::json(content.as_slice());

		// Assert
		assert_eq!(asset.content.as_ref(), content);
		assert_eq!(asset.mime_type, "application/json");
	}

	#[rstest]
	fn test_protocol_handler_register_and_resolve() {
		// Arrange
		let mut handler = ProtocolHandler::new();
		let content = b"<html><body>Hello</body></html>";

		// Act
		handler.register_asset("index.html", Asset::html(content.as_slice()));
		let resolved = handler.resolve("index.html");

		// Assert
		assert!(resolved.is_ok());
		let asset = resolved.unwrap();
		assert_eq!(asset.content.as_ref(), content);
		assert_eq!(asset.mime_type, "text/html");
	}

	#[rstest]
	fn test_protocol_handler_resolve_with_leading_slash() {
		// Arrange
		let mut handler = ProtocolHandler::new();
		handler.register_asset("styles.css", Asset::css(b"body {}".as_slice()));

		// Act
		let resolved = handler.resolve("/styles.css");

		// Assert
		assert!(resolved.is_ok());
		assert_eq!(resolved.unwrap().mime_type, "text/css");
	}

	#[rstest]
	fn test_protocol_handler_resolve_not_found() {
		// Arrange
		let handler = ProtocolHandler::new();

		// Act
		let resolved = handler.resolve("nonexistent.html");

		// Assert
		assert!(resolved.is_err());
	}

	#[rstest]
	#[case("html", "text/html")]
	#[case("htm", "text/html")]
	#[case("css", "text/css")]
	#[case("js", "application/javascript")]
	#[case("mjs", "application/javascript")]
	#[case("json", "application/json")]
	#[case("png", "image/png")]
	#[case("jpg", "image/jpeg")]
	#[case("jpeg", "image/jpeg")]
	#[case("gif", "image/gif")]
	#[case("svg", "image/svg+xml")]
	#[case("ico", "image/x-icon")]
	#[case("woff", "font/woff")]
	#[case("woff2", "font/woff2")]
	#[case("ttf", "font/ttf")]
	#[case("otf", "font/otf")]
	#[case("wasm", "application/wasm")]
	#[case("unknown", "application/octet-stream")]
	fn test_mime_type_for_extension(#[case] ext: &str, #[case] expected: &str) {
		// Act
		let mime = ProtocolHandler::mime_type_for_extension(ext);

		// Assert
		assert_eq!(mime, expected);
	}

	#[rstest]
	fn test_url_for() {
		// Act & Assert
		assert_eq!(
			ProtocolHandler::url_for("index.html"),
			"reinhardt://localhost/index.html"
		);
		assert_eq!(
			ProtocolHandler::url_for("/styles/main.css"),
			"reinhardt://localhost/styles/main.css"
		);
	}

	#[rstest]
	fn test_protocol_handler_register_bundled_css() {
		// Arrange
		let mut handler = ProtocolHandler::new();

		// Act
		handler.register_bundled_css("body { margin: 0; }");

		// Assert
		let css = handler.resolve("bundle.css").unwrap();
		assert_eq!(css.mime_type, "text/css");
		assert_eq!(
			std::str::from_utf8(&css.content).unwrap(),
			"body { margin: 0; }"
		);
	}

	#[rstest]
	fn test_protocol_handler_register_bundled_js() {
		// Arrange
		let mut handler = ProtocolHandler::new();

		// Act
		handler.register_bundled_js("console.log('init');");

		// Assert
		let js = handler.resolve("bundle.js").unwrap();
		assert_eq!(js.mime_type, "application/javascript");
		assert_eq!(
			std::str::from_utf8(&js.content).unwrap(),
			"console.log('init');"
		);
	}
}
