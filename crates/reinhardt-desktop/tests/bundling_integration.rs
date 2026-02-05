//! Integration tests for bundling.

use reinhardt_desktop::{
	ProtocolHandler,
	bundling::{AssetCollector, DesktopBundler},
};
use rstest::rstest;

#[rstest]
fn test_full_bundling_workflow() {
	// Arrange: Collect assets
	let mut collector = AssetCollector::new();
	collector.add_css(".header { font-size: 2rem; }", Some("header.css"));
	collector.add_css(".footer { padding: 1rem; }", Some("footer.css"));
	collector.add_js("function init() {}", Some("init.js"));
	collector.add_js("function main() { init(); }", Some("main.js"));

	// Act: Bundle assets
	let bundler = DesktopBundler::new();
	let bundled_css = bundler.bundle_css(&collector).unwrap();
	let bundled_js = bundler.bundle_js(&collector).unwrap();

	// Register with protocol handler
	let mut handler = ProtocolHandler::new();
	handler.register_bundled_css(&bundled_css);
	handler.register_bundled_js(&bundled_js);

	// Assert: Verify assets are accessible
	let css_asset = handler.resolve("bundle.css").unwrap();
	let js_asset = handler.resolve("bundle.js").unwrap();

	let css_content = std::str::from_utf8(&css_asset.content).unwrap();
	let js_content = std::str::from_utf8(&js_asset.content).unwrap();

	assert!(css_content.contains(".header"));
	assert!(css_content.contains(".footer"));
	assert!(js_content.contains("function init"));
	assert!(js_content.contains("function main"));
}

#[rstest]
fn test_bundling_preserves_order() {
	// Arrange
	let mut collector = AssetCollector::new();
	collector.add_css("/* first */", None);
	collector.add_css("/* second */", None);
	collector.add_css("/* third */", None);

	// Act
	let bundler = DesktopBundler::new();
	let bundled = bundler.bundle_css(&collector).unwrap();

	// Assert: CSS should be in order
	let first_pos = bundled.find("/* first */").unwrap();
	let second_pos = bundled.find("/* second */").unwrap();
	let third_pos = bundled.find("/* third */").unwrap();

	assert!(first_pos < second_pos);
	assert!(second_pos < third_pos);
}

#[rstest]
fn test_empty_collection_produces_empty_bundle() {
	// Arrange
	let collector = AssetCollector::new();
	let bundler = DesktopBundler::new();

	// Act
	let css = bundler.bundle_css(&collector).unwrap();
	let js = bundler.bundle_js(&collector).unwrap();

	// Assert
	assert!(css.is_empty());
	assert!(js.is_empty());
}

#[rstest]
fn test_bundling_with_protocol_handler_url() {
	// Arrange
	let mut collector = AssetCollector::new();
	collector.add_css("body { margin: 0; }", None);

	let bundler = DesktopBundler::new();
	let css = bundler.bundle_css(&collector).unwrap();

	let mut handler = ProtocolHandler::new();
	handler.register_bundled_css(&css);

	// Act
	let url = ProtocolHandler::url_for("bundle.css");

	// Assert
	assert_eq!(url, "reinhardt://localhost/bundle.css");
	assert!(handler.resolve("bundle.css").is_ok());
}
