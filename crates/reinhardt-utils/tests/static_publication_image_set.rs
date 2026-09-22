#![cfg(feature = "asset-publication")]

use reinhardt_utils::staticfiles::publication::{AssetInput, AssetMode, AssetPipeline};
use rstest::rstest;

#[rstest]
#[case("image-set")]
#[case("-webkit-image-set")]
#[case("IMAGE-SET")]
fn image_set_strings_are_dependencies_and_are_relocated(#[case] function: &str) {
	// Arrange
	let source = format!(
		r#"@media screen {{ .icon {{ background: {function}("../images/small.svg?v=1#mark" 1x type("image/svg+xml"), "../images/large.svg" 2x); content: "not-an-asset"; }} }}"#
	);
	let mut pipeline = AssetPipeline::new();
	for (name, bytes) in [
		("styles/main.css", source.as_bytes()),
		("images/small.svg", &b"<svg/>"[..]),
		("images/large.svg", &b"<svg width='2'/>"[..]),
	] {
		pipeline
			.add_input(AssetInput::bytes(name, bytes.to_vec()))
			.unwrap();
	}

	// Act
	let packed = pipeline.prepare(AssetMode::Production).unwrap();
	let css = String::from_utf8(packed.read_asset("styles/main.css").unwrap()).unwrap();

	// Assert
	assert_eq!(
		packed.manifest().assets["styles/main.css"].dependencies,
		["images/large.svg", "images/small.svg"]
	);
	assert!(css.contains("../../vectors/small.svg?v=1#mark"), "{css}");
	assert!(css.contains("../../vectors/large.svg"), "{css}");
	assert!(css.contains(r#"type("image/svg+xml")"#), "{css}");
	assert!(css.contains(r#"content: "not-an-asset""#), "{css}");
}

#[rstest]
fn image_set_preserves_external_strings_and_rewrites_url_candidates() {
	// Arrange
	let source = br#".icon { background: image-set("data:image/svg+xml;base64,PHN2Zy8+" 1x, "https://example.test/icon.svg" 2x, url("../images/local.svg") 3x); }"#;
	let mut pipeline = AssetPipeline::new();
	for (name, bytes) in [
		("styles/main.css", &source[..]),
		("images/local.svg", &b"<svg/>"[..]),
	] {
		pipeline
			.add_input(AssetInput::bytes(name, bytes.to_vec()))
			.unwrap();
	}

	// Act
	let packed = pipeline.prepare(AssetMode::Production).unwrap();
	let css = String::from_utf8(packed.read_asset("styles/main.css").unwrap()).unwrap();

	// Assert
	assert_eq!(
		packed.manifest().assets["styles/main.css"].dependencies,
		["images/local.svg"]
	);
	assert!(css.contains("data:image/svg+xml;base64,PHN2Zy8+"), "{css}");
	assert!(css.contains("https://example.test/icon.svg"), "{css}");
	assert!(css.contains("../../vectors/local.svg"), "{css}");
}
