#![cfg(feature = "asset-publication")]

use reinhardt_utils::staticfiles::publication::{
	AssetInput, AssetMode, AssetPipeline, AssetPublisher, analyze_asset_references,
};
use rstest::rstest;

#[rstest]
#[case(r#"<link rel="stylesheet" href="site.css" integrity="sha384-original">"#)]
#[case(r#"<script src="app.js" integrity="sha256-original"></script>"#)]
#[case(r#"<link integrity="sha384-original" rel="modulepreload" href="app.js">"#)]
#[case(
	r#"<link rel="stylesheet" href="{{ static_url('site.css') }}" integrity="sha384-original">"#
)]
fn local_integrity_is_rejected_before_publication(#[case] tag: &str) {
	// Arrange: both target resources have dependencies that change their published bytes.
	let mut pipeline = AssetPipeline::new();
	for (name, bytes) in [
		("site.css", &b"body{background:url('logo.svg')}"[..]),
		("logo.svg", b"<svg/>"),
		("app.js", b"import './dependency.js';"),
		("dependency.js", b"export const value = 1;"),
	] {
		pipeline
			.add_input(AssetInput::bytes(name, bytes.to_vec()))
			.unwrap();
	}
	pipeline
		.add_input(AssetInput::bytes(
			"index.html",
			format!("<!doctype html><html><head>{tag}</head><body></body></html>").into_bytes(),
		))
		.unwrap();

	// Act
	let error = pipeline.prepare(AssetMode::Production).unwrap_err();

	// Assert
	assert!(error.to_string().contains("integrity on local asset"));
	assert!(error.to_string().contains("index.html"));
}

#[rstest]
#[case("https://cdn.example.test/app.js")]
#[case("//cdn.example.test/app.js")]
#[case("data:text/javascript,void(0)")]
fn external_integrity_survives_html_rewriting(#[case] source: &str) {
	// Arrange: a local image forces the same HTML document through URL rewriting.
	let root = tempfile::tempdir().unwrap();
	let html = format!(
		r#"<html><head><script src="{source}" integrity="sha384-external"></script></head><body><img src="logo.svg"></body></html>"#
	);
	let mut pipeline = AssetPipeline::new();
	pipeline
		.add_input(AssetInput::bytes("index.html", html.into_bytes()))
		.unwrap();
	pipeline
		.add_input(AssetInput::bytes("logo.svg", b"<svg/>".to_vec()))
		.unwrap();

	// Act
	let snapshot = AssetPublisher::new(root.path().into())
		.publish(pipeline.prepare(AssetMode::Production).unwrap())
		.unwrap();
	let published = String::from_utf8(snapshot.read_asset("index.html").unwrap()).unwrap();

	// Assert
	assert!(published.contains(&format!("src=\"{source}\"")));
	assert!(published.contains("integrity=\"sha384-external\""));
	assert!(!published.contains("src=\"logo.svg\""));
	assert!(published.contains("../vectors/"));
}

#[rstest]
#[case("")]
#[case(" ")]
fn empty_integrity_does_not_block_local_assets(#[case] integrity: &str) {
	// Arrange
	let html = format!(r#"<link rel="stylesheet" href="site.css" integrity="{integrity}">"#);

	// Act
	let references = analyze_asset_references("index.html", "text/html", html.as_bytes()).unwrap();

	// Assert
	assert_eq!(references.len(), 1);
	assert_eq!(references[0].target, "site.css");
}
